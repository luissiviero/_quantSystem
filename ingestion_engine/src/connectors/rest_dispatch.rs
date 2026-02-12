// @file: ingestion_engine/src/connectors/rest_dispatch.rs
// @description: Generalized handler that routes requests to the correct exchange client.

use crate::core::models::{Exchange, MarketType, Candle};
use crate::core::interfaces::ExchangeRestClient;
use crate::connectors::binance_rest::BinanceRestClient;

// Factory function to get the correct client
fn get_client(exchange: Exchange) -> Box<dyn ExchangeRestClient> {
    match exchange {
        Exchange::Binance => Box::new(BinanceRestClient::new()),
        // Easy to add new exchanges here without breaking the server code
        Exchange::Bybit => panic!("Bybit REST not implemented"), 
        Exchange::Coinbase => panic!("Coinbase REST not implemented"),
    }
}

// The Generalized Entry Point
pub async fn fetch_history_generalized(
    exchange: Exchange,
    symbol: &str,
    market: MarketType,
    interval: &str,
    limit: usize
) -> Result<Vec<Candle>, String> {
    
    // 1. Get the specific implementation
    let client = get_client(exchange);
    
    // 2. Execute the trait method (Polymorphism)
    // The client guarantees the return type is Vec<Candle>
    client.fetch_history(symbol, market, interval, limit).await
}