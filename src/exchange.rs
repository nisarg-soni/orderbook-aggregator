use anyhow::{Context, Error, Result};

use crate::orderbook::{Level, Summary};

pub mod binance;
pub mod bitstamp;

// Basic orderbook struct for exchange response
#[derive(Clone, serde::Deserialize, Debug, serde::Serialize, Default)]
pub struct Orderbook {
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

impl Orderbook {
    // convert orderbook to Summary for GRPC
    pub fn convert(self, exchange: &str) -> Result<Summary> {
        let mut summary = Summary {
            bids: self
                .bids
                .iter()
                .take(10)
                .map(|b| Self::make_level(exchange, b))
                .collect::<Result<Vec<Level>, Error>>()
                .context("Failed to parse bids")?,
            asks: self
                .asks
                .iter()
                .take(10)
                .map(|b| Self::make_level(exchange, b))
                .collect::<Result<Vec<Level>, Error>>()
                .context("Failed to parse Asks")?,
            ..<_>::default()
        };

        Ok(summary)
    }

    fn make_level(exchange: &str, arr: &[String; 2]) -> Result<Level> {
        let price = arr[0]
            .parse::<f64>()
            .context("Failed to parse price for levels")?;
        let amount = arr[1]
            .parse::<f64>()
            .context("Failed to parse amount for levels")?;

        Ok(Level {
            exchange: exchange.to_string(),
            price,
            amount,
        })
    }
}
