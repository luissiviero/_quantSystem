// @file: src/tests/frontend_simulator.rs
// @description: Simulates frontend client behavior with time-based sampling and non-blocking metrics.
// @author: v5 helper

use crate::models::MarketData;
use crate::interfaces::DataProcessor;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

//
// STRUCT DEFINITION
//

#[allow(dead_code)]
pub struct FrontendSimulator {
    // We use atomic counter to track throughput without locking
    pub event_counter: AtomicU64,
    
    // Time-based sampling state (Unix Millis)
    pub last_trade_print: AtomicU64,
    pub last_agg_trade_print: AtomicU64,
    pub last_book_print: AtomicU64,
    pub last_candle_print: AtomicU64,
}

//
// IMPLEMENTATION
//

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

    // #1. Helper to check sampling interval and print
    fn check_and_print(&self, last_print: &AtomicU64, now: u64, data: &Arc<MarketData>, label: &str) {
        let last: u64 = last_print.load(Ordering::Relaxed);
        let interval: u64 = 10_000; // 10 seconds

        if now > last + interval {
            // Attempt to update the timestamp. If successful, we are the designated printer.
            let result: Result<u64, u64> = last_print.compare_exchange(
                last, 
                now, 
                Ordering::Relaxed, 
                Ordering::Relaxed
            );

            if result.is_ok() {
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
        let now: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // #3. Route based on type and check sampling interval
        match *data {
            MarketData::Trade(_) => {
                self.check_and_print(&self.last_trade_print, now, &data, "Trade");
            },
            MarketData::AggTrade(_) => {
                self.check_and_print(&self.last_agg_trade_print, now, &data, "AggTrade");
            },
            MarketData::OrderBook(_) => {
                self.check_and_print(&self.last_book_print, now, &data, "OrderBook");
            },
            MarketData::Candle(_) => {
                self.check_and_print(&self.last_candle_print, now, &data, "Candle");
            },
        }
    }

    fn on_error(&self, error: String) {
        eprintln!("[FrontendSimulator] Error: {}", error);
    }
}

//
// UNIT TESTS
//

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Trade, TradeSide};

    #[tokio::test]
    async fn test_simulator_logic() {
        let sim = FrontendSimulator::new();

        // #1. Create Data
        let trade = Trade {
            id: 100,
            symbol: "SOLUSDT".to_string(),
            price: 150.0,
            quantity: 10.0,
            timestamp_ms: 1000,
            side: TradeSide::Buy,
        };
        let data = Arc::new(MarketData::Trade(trade));

        // #2. Process Data
        sim.process(data.clone()).await;

        // #3. Verify the atomic counter increased
        let count = sim.event_counter.load(Ordering::Relaxed);
        assert_eq!(count, 1, "Simulator should have counted 1 event");
        
        println!("Frontend Simulator successfully processed {} event(s).", count);
    }
}