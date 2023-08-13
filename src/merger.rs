use anyhow::Result;
use tokio::sync::mpsc;

use crate::exchange;
use crate::orderbook::Summary;

pub async fn merger(symbol: String, binance_url: String, bitstamp_url: String) -> Result<()> {
    let (sender1, receiver1) = mpsc::channel(1);
    let (sender2, receiver2) = mpsc::channel(1);

    let sym_copy = symbol.clone();
    let binance_handle = tokio::spawn(async move {
        exchange::binance::BinanceExchange::start(symbol, binance_url, sender1).await
    });

    let bitstamp_handle = tokio::spawn(async move {
        exchange::bitstamp::BitstampExchange::start(sym_copy, bitstamp_url, sender2).await
    });

    tokio::spawn(async move {
        consumer_task(receiver1, receiver2).await;
    });

    binance_handle.await??;
    bitstamp_handle.await??;

    Ok(())
}

async fn consumer_task(
    mut binance_reciever: mpsc::Receiver<Summary>,
    mut bitstamp_reciever: mpsc::Receiver<Summary>,
) {
    loop {
        let (binance_msg, bitstamp_msg) =
            futures_util::join!(binance_reciever.recv(), bitstamp_reciever.recv());

        if binance_msg.is_some() && bitstamp_msg.is_some() {
            let merged = merge_summaries(&[binance_msg.unwrap(), bitstamp_msg.unwrap()]);

            println!("{:?}", merged);
        }
    }
}

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

    result
}
