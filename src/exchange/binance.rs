use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::Orderbook;
use crate::orderbook::Summary;

const EXCHANGE: &str = "BINANCE";

#[derive(Debug)]
pub struct BinanceExchange {}

impl BinanceExchange {
    pub async fn start(
        symbol: String,
        url: String,
        channel: mpsc::Sender<Summary>,
    ) -> anyhow::Result<()> {
        let url = url + "/ws/" + &symbol + "@depth10@100ms";

        loop {
            let res = connect_async(url.clone()).await;
            let (_, mut ws_read) = match res {
                Ok((stream, _)) => stream.split(),
                Err(e) => {
                    eprintln!("Failed to connect to Binance stream: {e}");
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    continue;
                }
            };

            while let Some(msg) = ws_read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(parsed) = serde_json::from_str::<Orderbook>(&text) {
                            match parsed.convert(EXCHANGE) {
                                Ok(summary) => {
                                    channel.send(summary).await?;
                                }
                                Err(_) => {
                                    eprintln!("Invalid Binance message format {text}");
                                }
                            }
                        } else {
                            eprintln!("Error parsing Binance orderbook");
                        }
                    }
                    _ => {
                        continue;
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(12)).await;
        }
    }
}
