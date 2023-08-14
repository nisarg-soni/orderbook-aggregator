use std::sync::{Arc, Mutex};
use tokio::sync::broadcast::{error::RecvError, Receiver, Sender};

use crate::orderbook::Summary;

#[derive(Debug)]
pub struct Merger {}

impl Merger {
    // recieve from both Binance and Bitstamp channels and merge and push to final channel whenever newer data comes in
    pub fn processor(
        mut binance_rec: Receiver<Summary>,
        mut bitstamp_rec: Receiver<Summary>,
        sender: Sender<Summary>,
    ) {
        let sender_bin = sender.clone();
        let sender_bit = sender.clone();

        let summaries = Arc::new(Mutex::new(vec![Summary::default(); 2]));
        let summaries_copy = Arc::clone(&summaries);

        // Listen to incoming messages from Binance
        tokio::spawn(async move {
            loop {
                match binance_rec.recv().await {
                    Ok(summary) => {
                        // lock summaries before reading and processing to avoid race conditions
                        let mut summaries = summaries.lock().unwrap();
                        summaries[0] = summary;
                        let merged = Self::merge_summaries(&summaries);
                        // Send merged summary to gRPC channel
                        _ = sender_bin.send(merged);
                    }
                    Err(RecvError::Lagged(_)) => {}
                    Err(RecvError::Closed) => {
                        eprintln!("Binance channel closed");
                    }
                }
            }
        });

        // Listen to incoming messages from Bitstamp
        tokio::spawn(async move {
            loop {
                match bitstamp_rec.recv().await {
                    Ok(summary) => {
                        // lock summaries before reading and processing to avoid race conditions
                        let mut summaries = summaries_copy.lock().unwrap();
                        summaries[1] = summary;
                        let merged = Self::merge_summaries(&summaries);
                        // Send merged summary to gRPC channel
                        _ = sender_bit.send(merged);
                    }
                    Err(RecvError::Lagged(_)) => {}
                    Err(RecvError::Closed) => {
                        eprintln!("Bitstamp channel closed");
                    }
                }
            }
        });
    }

    // sorts bids asks and discard after depth 10, also calculates spread
    fn merge_summaries(summaries: &[Summary]) -> Summary {
        let mut result = Summary::default();
        for s in summaries {
            result.bids.extend(s.bids.clone());
            result.asks.extend(s.asks.clone());
        }

        result.asks.sort_by(|first, second| {
            first
                .price
                .total_cmp(&second.price)
                .then(first.amount.total_cmp(&second.amount).reverse())
        });

        result.bids.sort_by(|first, second| {
            second
                .price
                .total_cmp(&first.price)
                .then(first.amount.total_cmp(&second.amount).reverse())
        });

        result.bids.truncate(10);
        result.asks.truncate(10);

        if !result.bids.is_empty() && !result.asks.is_empty() {
            result.spread = result.asks[0].price - result.bids[0].price;
        }

        // println!("{:?}", result);
        result
    }
}
