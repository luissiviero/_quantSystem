// @file: interfaces.rs
// @description: Defines traits for data processing and the FrontendSimulator.
// @author: v5 helper
// ingestion_engine/src/interfaces.rs

use crate::models::MarketData;
use async_trait::async_trait;
use std::sync::Arc;
// #1. FIX: Added missing imports for Atomic operations
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

//
// TRAIT DEFINITIONS
//

#[async_trait]
pub trait DataProcessor: Send + Sync {
    // #1. Process incoming market data
    // Using Arc<MarketData> to match the engine's zero-copy architecture
    async fn process(&self, data: Arc<MarketData>);
    
    // #2. Handle errors
    #[allow(dead_code)]
    fn on_error(&self, error: String);
}

//
// MOCK IMPLEMENTATION (Logger)
//

// #3. FIX: Suppress dead_code warning as this is a reference mock
#[allow(dead_code)]
pub struct LoggerProcessor;

#[async_trait]
impl DataProcessor for LoggerProcessor {
    async fn process(&self, data: Arc<MarketData>) {
        println!("Processor Log: {:?}", data);
    }

    fn on_error(&self, error: String) {
        eprintln!("Processor Error: {}", error);
    }
}


//
// FRONTEND SIMULATOR
//

// #4. FIX: Suppress dead_code warning until connected in main.rs
#[allow(dead_code)]
pub struct FrontendSimulator {
    // We use atomic counter to track throughput without locking
    pub event_counter: AtomicU64,
    // #5. Time-based sampling state (Unix Millis)
    pub last_trade_print: AtomicU64,
    pub last_agg_trade_print: AtomicU64,
    pub last_book_print: AtomicU64,
    pub last_candle_print: AtomicU64,
}

#[allow(dead_code)]
impl FrontendSimulator {
    pub fn new() -> Self {
        FrontendSimulator {
            event_counter: AtomicU64::new(0),
            last_trade_print: AtomicU64::new(0),
            last_agg_trade_print: AtomicU64::new(0),
            last_book_print: AtomicU64::new(0),
            last_candle_print: AtomicU64::new(0),
        }
    }

    // #6. Helper to check 10s interval and print
    fn check_and_print(&self, last_print: &AtomicU64, now: u64, data: &Arc<MarketData>, label: &str) {
        let last = last_print.load(Ordering::Relaxed);
        // 10,000 ms = 10 seconds
        if now > last + 10_000 {
            // Attempt to update the timestamp. If successful, we are the designated printer for this interval.
            // This avoids race conditions if multiple threads call process() simultaneously.
            if last_print.compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
                println!("[FrontendSimulator] 10s Sample [{}]: {:?}", label, data);
            }
        }
    }
}

#[async_trait]
impl DataProcessor for FrontendSimulator {
    async fn process(&self, data: Arc<MarketData>) {
        // #1. Count events (Internal stat)
        self.event_counter.fetch_add(1, Ordering::Relaxed);

        // #2. Get current time in millis
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // #3. Route based on type and check sampling interval
        match *data {
            MarketData::Trade(_) => self.check_and_print(&self.last_trade_print, now, &data, "Trade"),
            MarketData::AggTrade(_) => self.check_and_print(&self.last_agg_trade_print, now, &data, "AggTrade"),
            MarketData::OrderBook(_) => self.check_and_print(&self.last_book_print, now, &data, "OrderBook"),
            MarketData::Candle(_) => self.check_and_print(&self.last_candle_print, now, &data, "Candle"),
        }
    }

    fn on_error(&self, error: String) {
        eprintln!("[FrontendSimulator] Error: {}", error);
    }
}