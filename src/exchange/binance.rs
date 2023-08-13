use futures_util::StreamExt;
use tokio::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::Orderbook;

const EXCHANGE: &str = "BINANCE";

#[derive(Debug)]
pub struct BinanceExchange {}

impl BinanceExchange {
    pub async fn start(symbol: String, url: String) -> anyhow::Result<Self> {
        let url = url + "/ws/" + &symbol + "@depth10@100ms";

        loop {
            let res = connect_async(url.clone()).await;
            let (_, ws_read) = match res {
                Ok((stream, _)) => stream.split(),
                Err(e) => {
                    eprintln!("Failed to connect to Binance stream: {e}");
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    continue;
                }
            };

            let incoming_messages = ws_read.filter_map(|message| async {
                match message {
                    Ok(Message::Text(text)) => Some(text),
                    _ => None,
                }
            });

            incoming_messages
                .for_each(|msg| {
                    if let Ok(parsed) = serde_json::from_str::<Orderbook>(&msg) {
                        match parsed.convert(EXCHANGE) {
                            Ok(summary) => {
                                println!("{:?}", summary);
                            }
                            Err(_) => {
                                eprintln!("Invalid Binance message format `{msg}`");
                            }
                        }
                    } else {
                        eprintln!("Error parsing Binance orderbook");
                    }

                    async {}
                })
                .await;

            tokio::time::sleep(Duration::from_secs(12)).await;
        }
    }
}
