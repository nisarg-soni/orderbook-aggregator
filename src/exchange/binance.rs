use anyhow::Result;
use futures_util::StreamExt;
use tokio::sync::broadcast::Sender;
use tokio::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::Orderbook;
use crate::orderbook::Summary;

#[derive(Debug)]
pub struct BinanceExchange {}

impl BinanceExchange {
    // One process to fetch Binance exchange order books and push to channel
    pub async fn start(symbol: String, url: String, sender: Sender<Summary>) -> Result<()> {
        let url = url + "/ws/" + &symbol + "@depth10@100ms";

        let sender_copy = sender.clone();

        tokio::spawn(async move {
            loop {
                let (_, mut ws_read) = match connect_async(&url).await {
                    Ok((stream, _)) => stream.split(),
                    Err(err) => {
                        eprintln!("Binance connection failure: {err}");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                while let Some(msg) = ws_read.next().await {
                    let Ok(Message::Text(text)) = msg else { continue };
                    let Ok(data) = serde_json::from_str::<Orderbook>(&text) else { continue };
                    match data.convert("BINANCE") {
                        Ok(summary) => {
                            _ = sender_copy.send(summary);
                        }
                        Err(err) => {
                            eprintln!("Binance message parse failure: {err}");
                        }
                    }
                }
            }
        });

        println!("Binance connected");

        Ok(())
    }
}
