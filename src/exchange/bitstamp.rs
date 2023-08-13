use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::Orderbook;
use crate::orderbook::Summary;

const EXCHANGE: &str = "BITSTAMP";

#[derive(Debug)]
pub struct BitstampExchange {}

#[derive(Debug, serde::Deserialize)]
struct Data {
    pub data: Orderbook,
}

impl BitstampExchange {
    pub async fn start(
        symbol: String,
        url: String,
        channel: mpsc::Sender<Summary>,
    ) -> anyhow::Result<Self> {
        let subscription = r#"{"event":"bts:subscribe","data":{"channel":"order_book_"#.to_string()
            + &symbol
            + r#""}}"#;
        loop {
            let res = connect_async(url.clone()).await;
            let (mut ws_write, mut ws_read) = match res {
                Ok((stream, _)) => stream.split(),
                Err(e) => {
                    eprintln!("Failed to connect to Bitstamp stream: {e}");
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    continue;
                }
            };

            if let Err(err) = ws_write.send(Message::Text(subscription.clone())).await {
                eprintln!("Bitstamp subscription failed: {err}");
                continue;
            }

            while let Some(msg) = ws_read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(response) = serde_json::from_str::<serde_json::Value>(&text) {
                            if response["event"] == "data" {
                                if let Ok(parsed) = serde_json::from_value::<Data>(response) {
                                    match parsed.data.convert(EXCHANGE) {
                                        Ok(summary) => {
                                            channel.send(summary).await?;
                                        }
                                        Err(_) => {
                                            eprintln!("Invalid Bitstamp message format `{text}`");
                                        }
                                    }
                                } else {
                                    eprintln!("Bitstamp Exchange data parse error, {text}");
                                }
                            } else {
                                eprintln!("Bitstamp Exchange data parse error, {response}");
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
