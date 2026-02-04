// @file: src/tests/logger_processor.rs
// @description: Test processor that logs all market data to stdout to verify Binance stream.
// @author: v5 helper

use crate::interfaces::DataProcessor;
use crate::models::MarketData;
use crate::engine::Engine;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

//
// DASHBOARD STATE STRUCT
//

#[derive(Debug)]
struct DashboardState {
    last_trade: String,
    last_agg_trade: String,
    last_order_book: String,
    last_candle: String,
    
    // Integrity & Performance Metrics
    trade_count: u64,
    previous_trade_id: u64,
    gap_count: u64,
    
    // Latency Metrics
    // min_raw_delta: The smallest difference (Local - Remote) observed.
    // We assume this represents "Clock Skew + Physical Minimum Latency".
    min_raw_delta: i64,      
    is_baseline_set: bool,
    accumulated_jitter: i64, // Sum of (Current - Min)
    
    // UI Throttling
    last_ui_update_ms: u128,
}

impl Default for DashboardState {
    fn default() -> Self {
        DashboardState {
            last_trade: "Waiting...".to_string(),
            last_agg_trade: "Waiting...".to_string(),
            last_order_book: "Waiting...".to_string(),
            last_candle: "Waiting...".to_string(),
            trade_count: 0,
            previous_trade_id: 0,
            gap_count: 0,
            min_raw_delta: 0,
            is_baseline_set: false,
            accumulated_jitter: 0,
            last_ui_update_ms: 0,
        }
    }
}

//
// LOG PROCESSOR STRUCT
//

pub struct LogProcessor {
    state: Mutex<DashboardState>,
}

impl LogProcessor {
    pub fn new() -> Self {
        // Clear screen initially
        print!("\x1b[2J");
        LogProcessor {
            state: Mutex::new(DashboardState::default()),
        }
    }

    fn update_display(&self, state: &mut DashboardState) {
        let avg_jitter = if state.trade_count > 0 {
            state.accumulated_jitter as f64 / state.trade_count as f64
        } else {
            0.0
        };

        // ANSI Escape Codes:
        // \x1b[H  -> Move cursor to home (top-left)
        // \x1b[K  -> Clear line from cursor to end
        let output = format!(
            "\x1b[H\
            ================ QUANT SYSTEM INTEGRITY MONITOR ================\x1b[K\n\
            \n\
            [Integrity]\x1b[K\n\
            Trades Received: {}\x1b[K\n\
            Sequence Gaps:   {}  <-- Should be 0\x1b[K\n\
            \n\
            [Latency Analysis]\x1b[K\n\
            Clock Skew Est:  {} ms (Base difference)\x1b[K\n\
            Avg Jitter:      {:.2} ms (Delay above baseline)\x1b[K\n\
            \n\
            [Latest Data]\x1b[K\n\
            Trade:      {}\x1b[K\n\
            AggTrade:   {}\x1b[K\n\
            Order Book: {}\x1b[K\n\
            Klines:     {}\x1b[K\n\
            \n\
            ================================================================\x1b[K\n",
            state.trade_count,
            state.gap_count,
            state.min_raw_delta,
            avg_jitter,
            state.last_trade,
            state.last_agg_trade,
            state.last_order_book,
            state.last_candle
        );
        print!("{}", output);
    }
}

#[async_trait]
impl DataProcessor for LogProcessor {
    // #1. Process incoming data
    async fn process(&self, data: Arc<MarketData>) {
        let now_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        
        let should_redraw = {
            let mut state = self.state.lock().unwrap();
            
            match *data {
                MarketData::Trade(ref t) => {
                    // --- 1. Dynamic Calibration (Min-Delta Method) ---
                    let raw_diff = (now_ms as i64) - (t.timestamp_ms as i64);

                    // Initialize or Update Baseline if we find a "faster" packet
                    if !state.is_baseline_set || raw_diff < state.min_raw_delta {
                        state.min_raw_delta = raw_diff;
                        state.is_baseline_set = true;
                    }

                    // Jitter = How much slower is THIS packet compared to our best packet?
                    // This filters out the constant Clock Skew.
                    let jitter = raw_diff - state.min_raw_delta;
                    state.accumulated_jitter += jitter;

                    // --- 2. Sequence Gap Check ---
                    // Trade IDs must be sequential: 100, 101, 102...
                    if state.previous_trade_id != 0 {
                        if t.id != state.previous_trade_id + 1 {
                            state.gap_count += 1;
                        }
                    }
                    state.previous_trade_id = t.id;
                    state.trade_count += 1;

                    state.last_trade = format!("{} @ {:.2} (Qty: {:.4}) ID: {} [Lat: {}ms]", 
                        t.symbol, t.price, t.quantity, t.id, jitter);
                },
                MarketData::AggTrade(ref t) => {
                    state.last_agg_trade = format!("{} @ {:.2} (Qty: {:.4})", t.symbol, t.price, t.quantity);
                },
                MarketData::OrderBook(ref b) => {
                    state.last_order_book = format!("{} [UpdateID: {}] Bids: {} Asks: {}", 
                        b.symbol, b.last_update_id, b.bids.len(), b.asks.len());
                },
                MarketData::Candle(ref c) => {
                    state.last_candle = format!("{} [{}] O: {:.2} C: {:.2}", 
                        c.symbol, c.interval, c.open, c.close);
                },
            }

            // --- 3. UI Throttling ---
            // Only update screen every 100ms to avoid blocking the engine with I/O
            if now_ms > state.last_ui_update_ms + 100 {
                state.last_ui_update_ms = now_ms;
                true // release lock and redraw
            } else {
                false
            }
        };

        if should_redraw {
            if let Ok(mut state) = self.state.lock() {
                 self.update_display(&mut state);
            }
        }
    }

    fn on_error(&self, error: String) {
        eprintln!("\n\n>> [ERROR] LogProcessor: {}", error);
    }
}

//
// INTEGRATION TESTS
//

#[tokio::test]
async fn test_binance_stream() {
    // #1. Initialize Engine
    let engine: Engine = Engine::new();

    // #2. Register the Logger Processor
    engine.register_processor(Box::new(LogProcessor::new())).await;

    // #3. Define Symbol and Request Ingestion
    let symbol: String = "BTCUSDT".to_string();
    let is_new: bool = engine.request_ingestion(symbol.clone()).await;
    assert!(is_new, "Symbol should be new for this test instance");

    // #4. Spawn the Binance Connector
    let engine_clone: Engine = engine.clone();
    let symbol_clone: String = symbol.clone();
    
    tokio::spawn(async move {
        crate::exchanges::binance::connect_binance(symbol_clone, engine_clone).await;
    });

    // #5. Run for a fixed duration
    // 30 seconds to gather enough data for valid stats
    sleep(Duration::from_secs(30)).await;
}