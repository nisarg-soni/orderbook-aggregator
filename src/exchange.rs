use anyhow::{Result, Error};

use crate::orderbook::{Summary, Level};

pub mod binance;

#[derive(Clone, serde::Deserialize)]
pub struct Orderbook {
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

impl Orderbook {    
    pub fn convert(self, exchange: &str) -> Result<Summary> {
        let mut summary = Summary {
            bids: self
                .bids
                .iter()
                .take(10)
                .map(|b| {
                    Self::make_level(exchange, b)
                })
                .collect::<Result<Vec<Level>, Error>>()?,
            asks: self
                .asks
                .iter()
                .take(10)
                .map(|b| Self::make_level(exchange, b))
                .collect::<Result<Vec<Level>, Error>>()?,
            ..<_>::default()
        };

        if !summary.bids.is_empty() && !summary.asks.is_empty() {
            summary.spread = summary.asks[0].price - summary.bids[0].price;
        }

        Ok(summary)
    }

    fn make_level(exchange: &str, arr: &[String; 2]) -> Result<Level> {
        let price = arr[0].parse::<f64>()?;
        let amount = arr[1].parse::<f64>()?;

        Ok(Level {
            exchange: exchange.to_string(),
            price,
            amount,
        })
    }
}
