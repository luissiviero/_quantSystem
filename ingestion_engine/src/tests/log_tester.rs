// @file: ingestion_engine/src/tests/log_tester.rs
// @description: Test suite to verify selective stream ingestion (StreamConfig logic).
// @author: LAS.

use crate::core::interfaces::DataProcessor;
use crate::core::models::{MarketData, StreamConfig};
use crate::core::engine::Engine;
use crate::utils::config::AppConfig;
use crate::connectors::binance_spot; // Corrected path
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};


//
// VALIDATOR STRUCTURES
//

struct SharedValidator {
    // Maps data type (string) -> count (u64)
    pub counters: Arc<Mutex<HashMap<String, u64>>>,
}

#[async_trait]
impl DataProcessor for SharedValidator {
    async fn process(&self, data: Arc<MarketData>) {
        let mut counts = self.counters.lock().unwrap();
        
        // #1. Identify Data Type (Enhanced for Intervals)
        let key: String = match *data {
            MarketData::OrderBook(_) => "OrderBook".to_string(),
            MarketData::Trade(_) => "Trade".to_string(),
            MarketData::AggTrade(_) => "AggTrade".to_string(),
            MarketData::Candle(ref c) => format!("Candle({})", c.interval), // Track specific interval
            MarketData::HistoricalCandles(_) => "Historical".to_string(),
        };

        // #2. Increment Counter
        *counts.entry(key).or_insert(0) += 1;
    }

    fn on_error(&self, _: String) {
        // Ignore errors for this test
    }
}


//
// HELPERS
//

fn get_test_app_config() -> AppConfig {
    AppConfig {
        log_level: "info".to_string(),
        default_symbols: vec![],
        broadcast_buffer_size: 1000,
        trade_history_limit: 100,
        candle_history_limit: 100,
        binance_ws_url: "wss://stream.binance.com:9443/ws".to_string(),
        binance_reconnect_delay: 1,
        order_book_depth: "20".to_string(),
        default_raw_trades: true,
        default_agg_trades: true,
        default_order_book: true,
        default_kline_intervals: vec!["1m".to_string()],
        server_bind_address: "127.0.0.1:8080".to_string(),
        server_history_fetch_limit: 100,
    }
}

fn empty_stream_config() -> StreamConfig {
    StreamConfig {
        raw_trades: false,
        agg_trades: false,
        order_book: false,
        kline_intervals: vec![],
    }
}


//
// SCENARIO RUNNER
//

async fn test_configuration(stream_config: StreamConfig, description: &str) {
    println!("\n>> TESTING CONFIGURATION: {}", description);
    
    // #1. Setup Engine & Shared State
    let app_config = get_test_app_config();
    // Engine now requires config
    let engine: Engine = Engine::new(&app_config);
    
    let counters: Arc<Mutex<HashMap<String, u64>>> = Arc::new(Mutex::new(HashMap::new()));
    
    let processor: Box<SharedValidator> = Box::new(SharedValidator { 
        counters: counters.clone() 
    });
    
    engine.register_processor(processor).await;

    // #2. Start Ingestion
    // We use a high-volume pair (BTCUSDT) to ensure immediate data flow
    let symbol: String = "BTCUSDT".to_string();
    engine.request_ingestion(symbol.clone()).await;
    
    let engine_clone = engine.clone();
    let app_config_clone = app_config.clone();
    
    tokio::spawn(async move {
        // Corrected path and arguments
        binance_spot::connect_binance(symbol, engine_clone, stream_config, app_config_clone).await;
    });

    // #3. Collect Data
    // 5 seconds is usually enough to capture at least one event of each active type on BTC
    println!("   Collecting data for 5 seconds...");
    sleep(Duration::from_secs(5)).await;

    // #4. Report Results
    let final_counts = counters.lock().unwrap();
    println!("   Results for [{}]:", description);
    
    if final_counts.is_empty() {
        println!("   - [WARNING] No data received.");
    } else {
        // Sort keys for consistent output
        let mut sorted_keys: Vec<&String> = final_counts.keys().collect();
        sorted_keys.sort();
        
        for k in sorted_keys {
            println!("   - {:<15}: {}", k, final_counts[k]);
        }
    }
}


//
// MAIN TEST
//

#[tokio::test]
async fn verify_selective_streams() {
    // #1. Test Only Trades
    let mut cfg_trades = empty_stream_config();
    cfg_trades.raw_trades = true;
    
    test_configuration(cfg_trades, "ONLY Raw Trades").await;

    // #2. Test Only AggTrades
    let mut cfg_agg = empty_stream_config();
    cfg_agg.agg_trades = true;

    test_configuration(cfg_agg, "ONLY AggTrades").await;

    // #3. Test Only OrderBook
    let mut cfg_book = empty_stream_config();
    cfg_book.order_book = true;

    test_configuration(cfg_book, "ONLY OrderBook").await;
    
    // #4. Test Only 1m Klines (Specific Interval)
    let mut cfg_kline = empty_stream_config();
    cfg_kline.kline_intervals = vec!["1m".to_string()];

    test_configuration(cfg_kline, "ONLY 1m Klines").await;
}