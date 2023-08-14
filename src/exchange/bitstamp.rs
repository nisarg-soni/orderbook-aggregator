use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast::Sender;
use tokio::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::Orderbook;
use crate::orderbook::Summary;

#[derive(Debug)]
pub struct BitstampExchange {
    pub sender: Sender<Summary>,
}

impl BitstampExchange {
    pub async fn start(symbol: String, url: String, sender: Sender<Summary>) -> Result<Self> {
        let subscription = r#"{"event":"bts:subscribe","data":{"channel":"order_book_"#.to_string()
            + &symbol
            + r#""}}"#;

        let sender_copy = sender.clone();

        tokio::spawn(async move {
            loop {
                let (mut ws_write, mut ws_read) = match connect_async(&url).await {
                    Ok((stream, _)) => stream.split(),
                    Err(err) => {
                        eprintln!("Bitstamp connection failure: {err}");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                };

                if let Err(err) = ws_write.send(Message::Text(subscription.clone())).await {
                    eprintln!("Bitstamp subscription failure: {err}");
                    continue;
                }

                while let Some(msg) = ws_read.next().await {
                    let Ok(Message::Text(text)) = msg else { continue };
                    let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
                    if val["event"] != "data" {
                        continue;
                    }
                    let Ok(data) = serde_json::from_value::<Data>(val) else { continue };
                    match data.data.convert("BITSTAMP") {
                        Ok(summary) => {
                            _ = sender_copy.send(summary);
                        }
                        Err(err) => {
                            eprintln!("Bitstamp message parse failure: {err}");
                        }
                    }
                }
            }
        });

        println!("Bitstamp connected");

        Ok(BitstampExchange { sender })
    }
}

#[derive(Debug, serde::Deserialize)]
struct Data {
    pub data: Orderbook,
}
