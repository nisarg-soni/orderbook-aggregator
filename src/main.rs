use merger::Merger;
use orderbook::orderbook_aggregator_server::{OrderbookAggregator, OrderbookAggregatorServer};
use orderbook::{Empty, Summary};

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

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(long, value_parser, default_value = "ethbtc")]
    trade_pair: String,

    #[clap(long, value_parser, default_value = "wss://stream.binance.com:9443")]
    binance_url: String,

    #[clap(long, value_parser, default_value = "wss://ws.bitstamp.net")]
    bitstamp_url: String,
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let (sender, _) = channel(1);

    let (sender1, _) = channel(1);
    let (sender2, _) = channel(1);

    let sym_copy = cli.trade_pair.clone();
    let (_, _) = futures_util::try_join!(
        exchange::binance::BinanceExchange::start(cli.trade_pair, cli.binance_url, sender1.clone()),
        exchange::bitstamp::BitstampExchange::start(sym_copy, cli.bitstamp_url, sender2.clone()),
    )?;

    let _ = Merger::processor(sender1.subscribe(), sender2.subscribe(), sender.clone());

    let server = OrderbookAggregatorServer::new(GRPC { sender });

    tonic::transport::Server::builder()
        .add_service(server)
        .serve(std::net::SocketAddr::from(([127, 0, 0, 1], 7016)))
        .await?;
    Ok(())
}

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

        let out = BroadcastStream::new(receiever).filter_map(|r| {
            std::future::ready(match r {
                Ok(r) => Some(Ok::<_, _>(r)),
                _ => None,
            })
        });

        Ok(tonic::Response::new(
            Box::pin(out) as Self::BookSummaryStream
        ))
    }
}
