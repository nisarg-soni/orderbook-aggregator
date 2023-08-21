#[cfg(test)]
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use std::{
    net::SocketAddr,
    sync::{
        atomic::{self, AtomicU64},
        Arc, RwLock,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::exchange::Orderbook;
use crate::orderbook::{orderbook_aggregator_client::OrderbookAggregatorClient, Empty};
use crate::run;

#[cfg(test)]
pub struct MockBinance {
    data: Arc<RwLock<Orderbook>>,
    port: u16,
}

#[cfg(test)]
impl MockBinance {
    pub fn start() -> Self {
        let data = Arc::default();

        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let app = Router::new()
            .route("/ws/ethbtc@depth10@100ms", get(Self::ws_handler))
            .with_state(Arc::clone(&data));
        let server = axum::Server::bind(&addr)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>());
        let port = server.local_addr().port();

        tokio::spawn(async move {
            eprintln!("MockBinance listening on {}", server.local_addr());
            server.await.unwrap();
        });

        Self { data, port }
    }

    pub fn url(&self) -> String {
        format!("ws://localhost:{}", self.port)
    }

    pub fn set_orders(&self, book: Orderbook) {
        *self.data.write().unwrap() = book;
    }

    async fn ws_handler(
        ws: WebSocketUpgrade,
        State(data): State<Arc<RwLock<Orderbook>>>,
    ) -> impl IntoResponse {
        ws.on_upgrade(|ws| Self::connect_ws(ws, data))
    }

    async fn connect_ws(mut ws: WebSocket, data: Arc<RwLock<Orderbook>>) {
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        eprintln!("MockBinance publishing on new connection");

        loop {
            let msg = {
                let data = data.read().unwrap();
                serde_json::json!({
                    "lastUpdateId": COUNTER.fetch_add(1, atomic::Ordering::Relaxed),
                    "bids": data.bids.iter().collect::<Vec<_>>(),
                    "asks": data.asks.iter().collect::<Vec<_>>(),
                })
            };

            if ws
                .send(Message::Text(serde_json::to_string(&msg).unwrap()))
                .await
                .is_err()
            {
                return;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

#[cfg(test)]
pub struct MockBitstamp {
    data: Arc<RwLock<Orderbook>>,
    port: u16,
}

#[cfg(test)]
impl MockBitstamp {
    pub fn start() -> Self {
        let data = Arc::default();

        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let app = Router::new()
            .route("/", get(Self::ws_handler))
            .with_state(Arc::clone(&data));
        let server = axum::Server::bind(&addr)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>());
        let port = server.local_addr().port();

        tokio::spawn(async move {
            eprintln!("MockBitstamp listening on {}", server.local_addr());
            server.await.unwrap();
        });

        Self { data, port }
    }

    pub fn url(&self) -> String {
        format!("ws://localhost:{}", self.port)
    }

    pub fn set_orders(&self, book: Orderbook) {
        *self.data.write().unwrap() = book;
    }

    async fn ws_handler(
        ws: WebSocketUpgrade,
        State(data): State<Arc<RwLock<Orderbook>>>,
    ) -> impl IntoResponse {
        ws.on_upgrade(|ws| Self::connect_ws(ws, data))
    }

    async fn connect_ws(mut ws: WebSocket, data: Arc<RwLock<Orderbook>>) {
        eprintln!("MockBitstamp new connection");

        loop {
            if let Some(msg) = ws.recv().await {
                if let Ok(Message::Text(json)) = msg {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json) {
                        if json["event"] == "bts:subscribe"
                            && json["data"]["channel"] == "order_book_ethbtc"
                        {
                            break;
                        }
                    }
                }
            } else {
                return;
            }
        }

        eprintln!("MockBitstamp publishing on new connection");

        loop {
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let msg = {
                let data = data.read().unwrap();
                serde_json::json!({
                    "data": {
                        "timestamp": timestamp.as_secs().to_string(),
                        "microtimestamp": timestamp.as_micros().to_string(),
                        "bids": data.bids.iter().collect::<Vec<_>>(),
                        "asks": data.asks.iter().collect::<Vec<_>>(),
                    },
                    "channel": "order_book_ethbtc",
                    "event": "data",
                })
            };

            if ws
                .send(Message::Text(serde_json::to_string(&msg).unwrap()))
                .await
                .is_err()
            {
                return;
            }

            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
}

#[cfg(test)]
macro_rules! assert_level_eq {
    ($lvl:expr, $name:expr, $price:expr, $amount:expr) => {{
        let lvl = &$lvl;
        assert_eq!(lvl.exchange, $name);
        assert_eq!(lvl.price, $price);
        assert_eq!(lvl.amount, $amount);
    }};
}

#[cfg(test)]
#[tokio::test]
async fn test() {
    let binance = MockBinance::start();
    binance.set_orders(Orderbook {
        bids: vec![
            ["100.0".into(), "5.0".into()],
            ["99.0".into(), "10.0".into()],
        ],
        asks: vec![
            ["104.0".into(), "9.0".into()],
            ["106.0".into(), "7.0".into()],
        ],
    });

    let bitstamp = MockBitstamp::start();
    bitstamp.set_orders(Orderbook {
        bids: vec![["101".into(), "9.0".into()], ["98".into(), "12.0".into()]],
        asks: vec![["103".into(), "4.0".into()], ["105".into(), "8.0".into()]],
    });

    let binance_url = binance.url();
    let bitstamp_url = bitstamp.url();

    tokio::spawn(async {
        if let Err(err) = run("ethbtc".into(), binance_url, bitstamp_url, "8091".into()).await {
            eprintln!("{err}");
        }
    });

    let mut client = loop {
        let c = OrderbookAggregatorClient::connect("http://localhost:8091").await;
        if let Ok(client) = c {
            break client;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };

    let mut stream = client
        .book_summary(Empty {})
        .await
        .expect("book_summary")
        .into_inner();

    let msg = loop {
        let next = stream.message().await.unwrap();
        let next = next.expect("stream closed");

        if next.bids.iter().any(|b| b.exchange == "BINANCE")
            && next.bids.iter().any(|b| b.exchange == "BITSTAMP")
        {
            break next;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };

    let bids = &msg.bids;
    let asks = &msg.asks;
    // println!("BIDS: {:?}", bids);
    // println!("ASKS: {:?}", asks);

    assert_level_eq!(bids[0], "BITSTAMP", 101.0, 9.0);
    assert_level_eq!(bids[1], "BINANCE", 100.0, 5.0);
    assert_level_eq!(bids[2], "BINANCE", 99.0, 10.0);
    assert_level_eq!(bids[3], "BITSTAMP", 98.0, 12.0);

    assert_level_eq!(asks[0], "BITSTAMP", 103.0, 4.0);
    assert_level_eq!(asks[1], "BINANCE", 104.0, 9.0);
    assert_level_eq!(asks[2], "BITSTAMP", 105.0, 8.0);
    assert_level_eq!(asks[3], "BINANCE", 106.0, 7.0);

    assert_eq!(msg.spread, 103.0 - 101.0);
}
