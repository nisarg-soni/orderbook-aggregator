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
        let mut summaries: Vec<Summary> = vec![Summary::default(); 2];

        tokio::spawn(async move {
            loop {
                // await futures for the first new summary recieved from Binance or Bitstamp
                tokio::select! {
                    summary = binance_rec.recv() => {
                        match summary {
                            Ok(summary) => {
                                summaries[0] = summary;
                                let merged = Self::merge_summaries(&summaries);
                                // Send merged summary to gRPC channel
                                _ = sender.send(merged);
                            }
                            Err(RecvError::Lagged(_)) => {}
                            Err(RecvError::Closed) => {
                                eprintln!("Binance channel closed");
                            }
                        }

                    },
                    summary = bitstamp_rec.recv() => {
                        match summary {
                            Ok(summary) => {
                                summaries[1] = summary;
                                let merged = Self::merge_summaries(&summaries);
                                // Send merged summary to gRPC channel
                                _ = sender.send(merged);
                            }
                            Err(RecvError::Lagged(_)) => {}
                            Err(RecvError::Closed) => {
                                eprintln!("Bitstamp channel closed");
                            }
                        }
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
