
use clap::Parser;

pub mod exchange;

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

    let _ = exchange::binance::BinanceExchange::start(cli.trade_pair, cli.binance_url).await?;

    Ok(())
}

