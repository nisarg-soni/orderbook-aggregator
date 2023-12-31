use merger::Merger;
use orderbook::orderbook_aggregator_server::{OrderbookAggregator, OrderbookAggregatorServer};
use orderbook::{Empty, Summary};

use anyhow::Context;
use clap::Parser;
use futures_util::{Stream, StreamExt};
use std::pin::Pin;
use tokio::sync::broadcast::{channel, Sender};
use tokio_stream::wrappers::BroadcastStream;
use tonic::{Request, Response, Status};

pub mod exchange;
pub mod merger;

pub mod orderbook {
    tonic::include_proto!("orderbook");
}

#[cfg(test)]
pub mod tests;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    // Required trade pair
    #[clap(long, value_parser, default_value = "ethbtc")]
    trade_pair: String,

    // URL for ws connection from Binance
    #[clap(long, value_parser, default_value = "wss://stream.binance.com:9443")]
    binance_url: String,

    // URL for ws connection from bitstamp
    #[clap(long, value_parser, default_value = "wss://ws.bitstamp.net")]
    bitstamp_url: String,

    // Port for gRPC server
    #[clap(long, value_parser, default_value = "7050")]
    port: String,
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    run(cli.trade_pair, cli.binance_url, cli.bitstamp_url, cli.port).await?;

    Ok(())
}

pub async fn run(
    trade_pair: String,
    binance_url: String,
    bitstamp_url: String,
    port: String,
) -> anyhow::Result<()> {
    // Channel for merged orderbooks
    let (sender, _) = channel(1);

    // Channels for binance orderbooks and bitstamp orderooks
    let (sender1, _) = channel(1);
    let (sender2, _) = channel(1);

    // Start receiving from Binance
    exchange::binance::BinanceExchange::start(trade_pair.clone(), binance_url, sender1.clone())
        .await
        .context("Failed to start Binance receiver")?;

    // Start receiving from Bitstamp
    exchange::bitstamp::BitstampExchange::start(trade_pair, bitstamp_url, sender2.clone())
        .await
        .context("Failed to start Bitstamp receiver")?;

    Merger::processor(sender1.subscribe(), sender2.subscribe(), sender.clone());

    let server = OrderbookAggregatorServer::new(GRPC { sender });

    // Start GRPC server
    let port = port.parse::<u16>().unwrap_or(7050);
    tonic::transport::Server::builder()
        .add_service(server)
        .serve(std::net::SocketAddr::from(([127, 0, 0, 1], port)))
        .await
        .context("Failed to satrt gRPC server")?;

    Ok(())
}

// GRPC server method implementation
#[derive(Debug)]
pub struct GRPC {
    sender: Sender<Summary>,
}

#[tonic::async_trait]
impl OrderbookAggregator for GRPC {
    type BookSummaryStream = Pin<Box<dyn Stream<Item = Result<Summary, Status>> + Send>>;

    async fn book_summary(
        &self,
        _: Request<Empty>,
    ) -> Result<Response<Self::BookSummaryStream>, Status> {
        let receiever = self.sender.subscribe();

        // Conversion of Receiver<Summary> into Stream<Receiver<Result<Summary, Status>>>
        let result = BroadcastStream::new(receiever).filter_map(|r| {
            std::future::ready(match r {
                Ok(r) => Some(Ok::<_, _>(r)),
                _ => None,
            })
        });

        Ok(tonic::Response::new(
            Box::pin(result) as Self::BookSummaryStream
        ))
    }
}
