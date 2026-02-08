// @file: ingestion_engine/src/connectors/mod.rs
// @description: Factory module for spawning connector tasks based on Exchange/MarketType.
// @author: LAS.

pub mod binance;

use crate::core::models::{Exchange, MarketType, StreamConfig};
use crate::core::engine::Engine;
use crate::utils::config::AppConfig;
use tokio::task;

//
// FACTORY FUNCTION
//

pub async fn spawn_connector(
    exchange: Exchange,
    market_type: MarketType,
    symbol: String,
    engine: Engine,
    stream_config: StreamConfig,
    app_config: AppConfig
) {
    // #1. Construct Unique ID (Namespacing)
    // We prefix the symbol so the engine stores "BINANCE_SPOT_BTCUSDT"
    // This prevents collisions if the same symbol exists on multiple exchanges.
    let unique_id: String = format!("{}_{}_{}", exchange, market_type, symbol).to_uppercase();

    // #2. Dispatch to specific implementation
    match exchange {
        Exchange::Binance => {
            task::spawn(async move {
                binance::connect_binance(
                    symbol, // Pass original symbol to connector
                    unique_id, // Pass unique ID for Engine storage
                    market_type, 
                    engine, 
                    stream_config, 
                    app_config
                ).await;
            });
        }
        
        Exchange::Bybit => {
            println!("Bybit connector not implemented yet.");
        }
        
        Exchange::Coinbase => {
            println!("Coinbase connector not implemented yet.");
        }
    }
}