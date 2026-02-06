// @file: ingestion_engine/src/utils/config.rs
// @description: Centralized application configuration handling using config crate.
// @author: LAS.

use serde::Deserialize;
use config::{Config, ConfigError, File, Environment};
use crate::core::models::StreamConfig;

//
// TYPE DEFINITIONS
//

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub log_level: String,
    pub default_symbols: Vec<String>,
    
    // Engine Limits
    pub broadcast_buffer_size: usize,
    pub trade_history_limit: usize,
    pub candle_history_limit: usize,

    // Binance Settings
    pub binance_ws_url: String,
    pub binance_reconnect_delay: u64,
    pub order_book_depth: String,

    // Stream Defaults
    pub default_raw_trades: bool,
    pub default_agg_trades: bool,
    pub default_order_book: bool,
    pub default_kline_intervals: Vec<String>,

    // Server Settings
    pub server_bind_address: String,
    pub server_history_fetch_limit: usize,
}

impl AppConfig {
    //
    // PUBLIC INTERFACE
    //

    pub fn load() -> Result<Self, ConfigError> {
        let builder = Config::builder()
            .set_default("log_level", "info")?
            .set_default("default_symbols", vec!["BTCUSDT"])?
            .set_default("broadcast_buffer_size", 5000)?
            .set_default("trade_history_limit", 100)?
            .set_default("candle_history_limit", 5000)?
            // Default Binance Settings
            .set_default("binance_ws_url", "wss://stream.binance.com:9443/ws")?
            .set_default("binance_reconnect_delay", 60)?
            .set_default("order_book_depth", "20")?
            // Stream Defaults
            .set_default("default_raw_trades", true)?
            .set_default("default_agg_trades", true)?
            .set_default("default_order_book", true)?
            .set_default("default_kline_intervals", vec![
                "1m", "5m", "15m", "1h", "4h", "1d"
            ])?
            // Server Defaults
            .set_default("server_bind_address", "127.0.0.1:8080")?
            .set_default("server_history_fetch_limit", 1000)?
            // File & Env Overrides
            .add_source(File::with_name("config").required(false))
            .add_source(Environment::with_prefix("APP"));

        let config = builder.build()?;
        config.try_deserialize()
    }

    pub fn get_stream_config(&self) -> StreamConfig {
        StreamConfig {
            raw_trades: self.default_raw_trades,
            agg_trades: self.default_agg_trades,
            order_book: self.default_order_book,
            kline_intervals: self.default_kline_intervals.clone(),
        }
    }
}