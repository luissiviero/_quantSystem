// @file: src/tests/log_tester.rs
// @description: Test suite to verify selective stream ingestion (StreamConfig logic).
// @author: v5 helper
// ingestion_engine/src/tests/log_tester.rs

use crate::interfaces::DataProcessor;
use crate::models::{MarketData, StreamConfig};
use crate::engine::Engine;
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
// SCENARIO RUNNER
//

async fn test_configuration(config: StreamConfig, description: &str) {
    println!("\n>> TESTING CONFIGURATION: {}", description);
    
    // #1. Setup Engine & Shared State
    let engine: Engine = Engine::new();
    let counters: Arc<Mutex<HashMap<String, u64>>> = Arc::new(Mutex::new(HashMap::new()));
    
    let processor: Box<SharedValidator> = Box::new(SharedValidator { 
        counters: counters.clone() 
    });
    
    engine.register_processor(processor).await;

    // #2. Start Ingestion
    // We use a high-volume pair (BTCUSDT) to ensure immediate data flow
    let symbol: String = "BTCUSDT".to_string();
    engine.request_ingestion(symbol.clone()).await;
    
    let engine_clone: Engine = engine.clone();
    
    tokio::spawn(async move {
        crate::exchanges::binance::connect_binance(symbol, engine_clone, config).await;
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
    let mut cfg_trades: StreamConfig = StreamConfig::default();
    cfg_trades.raw_trades = true;
    cfg_trades.agg_trades = false;
    cfg_trades.order_book = false;
    cfg_trades.kline_intervals = vec![]; // Disable klines
    
    test_configuration(cfg_trades, "ONLY Raw Trades").await;

    // #2. Test Only AggTrades
    let mut cfg_agg: StreamConfig = StreamConfig::default();
    cfg_agg.raw_trades = false;
    cfg_agg.agg_trades = true;
    cfg_agg.order_book = false;
    cfg_agg.kline_intervals = vec![];

    test_configuration(cfg_agg, "ONLY AggTrades").await;

    // #3. Test Only OrderBook
    // Note: OrderBooks snapshot immediately, but updates might be slower
    let mut cfg_book: StreamConfig = StreamConfig::default();
    cfg_book.raw_trades = false;
    cfg_book.agg_trades = false;
    cfg_book.order_book = true;
    cfg_book.kline_intervals = vec![];

    test_configuration(cfg_book, "ONLY OrderBook").await;
    
    // #4. Test Only 1m Klines (Specific Interval)
    let mut cfg_kline: StreamConfig = StreamConfig::default();
    cfg_kline.raw_trades = false;
    cfg_kline.agg_trades = false;
    cfg_kline.order_book = false;
    cfg_kline.kline_intervals = vec!["1m".to_string()]; // Request ONLY 1m

    test_configuration(cfg_kline, "ONLY 1m Klines").await;
}