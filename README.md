# Orderbook Aggregator

GRPC server to fetch orderbooks from crypto exchanges, sort, merge and publish the resultant orderbook. 
## Current supported exchanges:

 - Binance
 - Bitstamp
## Run

    cargo run --release -- --trade-pair <trade_pair> --binance-url <binance_url> --bitstamp-url <bitstamp_url> --port <grpc_port>
    
**Parameters:**

 - `trade_pair` : trade pair symbols. Should be same and available on both exchanges. **Default : ethbtc**
 - `binance_url` : URL for the websocket connection for Binance. **Default: `wss://stream.binance.com:9443`**
 - - `bitstamp_url` : URL for the websocket connection for Bitstamp. **Default: `wss://ws.bitstamp.net`**
 - `grpc_port` : Port number for GRPC server **Default : `7050`**

## Notes

 - The program serve merged order books if one of the exchange does not provide order book for the trade pair, then order book from only one exchange will be served. This also happens, up till the first order book is received from both the exchanges.
 - Currently, the server runs for a single trade pair at a time.
 - `orderbook.proto` contains the defination of the message format.
