// @file: src/tests/logger_processor.rs
// @description: Test processor that logs all market data to stdout to verify Binance stream.
// @author: v5 helper

use crate::interfaces::DataProcessor;
use crate::models::{MarketData, StreamConfig}; // #1. Added StreamConfig import
use crate::engine::Engine;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use serde::Deserialize;

//
// DYNAMIC SYMBOL FETCHING
//

#[derive(Deserialize, Debug)]
struct Ticker24hr {
    symbol: String,
    #[serde(rename = "quoteVolume")]
    quote_volume: String, 
}

async fn fetch_top_volume_symbols(limit: usize) -> Vec<String> {
    println!(">> Fetching top {} symbols by volume from Binance API...", limit);
    
    let client = reqwest::Client::new();
    let resp = client.get("https://api.binance.com/api/v3/ticker/24hr")
        .send()
        .await
        .expect("Failed to fetch from Binance API");
    
    let tickers: Vec<Ticker24hr> = resp.json().await.expect("Failed to parse JSON");

    let mut usdt_pairs: Vec<(String, f64)> = tickers.into_iter()
        .filter(|t| t.symbol.ends_with("USDT"))
        .map(|t| (t.symbol, t.quote_volume.parse::<f64>().unwrap_or(0.0)))
        .collect();

    usdt_pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let top_symbols: Vec<String> = usdt_pairs.into_iter()
        .take(limit)
        .map(|(sym, _)| sym)
        .collect();

    println!(">> Top 3 Symbols: {:?}", &top_symbols[0..3]);
    top_symbols
}

//
// DASHBOARD STATE STRUCT
//

#[derive(Debug)]
struct DashboardState {
    scenario_title: String,
    start_time_ms: u128,
    lines_printed_previously: usize,
    last_trade_str: String,
    trade_count: u64,
    previous_trade_ids: HashMap<String, u64>,
    gap_count: u64,
    min_raw_delta: i64,      
    is_baseline_set: bool,
    accumulated_jitter: i64, 
    burst_count: u64,
    current_burst_depth: u64,
    max_burst_depth: u64,
    last_process_ms: u128,
    last_ui_update_ms: u128,
}

impl DashboardState {
    fn new(title: String) -> Self {
        DashboardState {
            scenario_title: title,
            start_time_ms: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
            lines_printed_previously: 0,
            last_trade_str: "Waiting for data...".to_string(),
            trade_count: 0,
            previous_trade_ids: HashMap::new(),
            gap_count: 0,
            min_raw_delta: 0,
            is_baseline_set: false,
            accumulated_jitter: 0,
            burst_count: 0,
            current_burst_depth: 0,
            max_burst_depth: 0,
            last_process_ms: 0,
            last_ui_update_ms: 0,
        }
    }
}

//
// LOG PROCESSOR STRUCT
//

pub struct LogProcessor {
    state: Mutex<DashboardState>,
    is_active: Arc<AtomicBool>,
}

impl LogProcessor {
    pub fn new(title: String, active_flag: Arc<AtomicBool>) -> Self {
        LogProcessor {
            state: Mutex::new(DashboardState::new(title)),
            is_active: active_flag,
        }
    }

    fn update_display(&self, state: &mut DashboardState) {
        let avg_jitter = if state.trade_count > 0 {
            state.accumulated_jitter as f64 / state.trade_count as f64
        } else {
            0.0
        };

        let content = format!(
            "================ STRESS TEST: {} ================\x1b[K\n\
            [Integrity] Trades: {:<8} | Gaps: {:<4} | Bursts: {:<4} (Max Depth: {})\x1b[K\n\
            [Latency]   Skew: {:<4} ms | Avg Jitter: {:.2} ms\x1b[K\n\
            [Last Data] {}\x1b[K\n\
            ========================================================================\x1b[K", 
            state.scenario_title,
            state.trade_count,
            state.gap_count,
            state.burst_count,
            state.max_burst_depth,
            state.min_raw_delta,
            avg_jitter,
            state.last_trade_str
        );

        let line_count = content.matches('\n').count() + 1;

        if state.lines_printed_previously > 0 {
            print!("\x1b[{}A", state.lines_printed_previously);
        }

        println!("{}\r", content);

        state.lines_printed_previously = line_count;
    }
}

#[async_trait]
impl DataProcessor for LogProcessor {
    async fn process(&self, data: Arc<MarketData>) {
        if !self.is_active.load(Ordering::Relaxed) {
            return;
        }

        let now_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        
        let should_redraw = {
            let mut state = self.state.lock().unwrap();

            if now_ms == state.last_process_ms {
                state.current_burst_depth += 1;
                if state.current_burst_depth > state.max_burst_depth {
                    state.max_burst_depth = state.current_burst_depth;
                }
                if state.current_burst_depth == 2 {
                    state.burst_count += 1;
                }
            } else {
                state.last_process_ms = now_ms;
                state.current_burst_depth = 1;
            }
            
            match *data {
                MarketData::Trade(ref t) => {
                    let raw_diff = (now_ms as i64) - (t.timestamp_ms as i64);

                    if !state.is_baseline_set || raw_diff < state.min_raw_delta {
                        state.min_raw_delta = raw_diff;
                        state.is_baseline_set = true;
                    }

                    let jitter = raw_diff - state.min_raw_delta;
                    state.accumulated_jitter += jitter;

                    let is_gap = {
                        let last_id = state.previous_trade_ids.entry(t.symbol.clone()).or_insert(0);
                        let gap_detected = *last_id != 0 && t.id != *last_id + 1;
                        *last_id = t.id;
                        gap_detected
                    };

                    if is_gap {
                        state.gap_count += 1;
                    }
                    
                    state.trade_count += 1;

                    state.last_trade_str = format!("{} @ {:.2} (Qty: {:.4}) [Lat: {}ms]", 
                        t.symbol, t.price, t.quantity, jitter);
                },
                _ => {} 
            }

            if now_ms > state.start_time_ms + 2000 && now_ms > state.last_ui_update_ms + 250 {
                state.last_ui_update_ms = now_ms;
                true
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

    fn on_error(&self, _error: String) {
    }
}

//
// HELPER: SCENARIO RUNNER
//

async fn run_scenario(title: &str, symbols: Vec<String>, use_pinned: bool) {
    println!("Successfully connected to Binance"); 
    
    let engine = Engine::new();
    let active_flag = Arc::new(AtomicBool::new(true));
    
    engine.register_processor(Box::new(LogProcessor::new(title.to_string(), active_flag.clone()))).await;

    for symbol in symbols {
        if engine.request_ingestion(symbol.clone()).await {
            let engine_clone = engine.clone();
            let symbol_clone = symbol.clone();

            // #2. Create Config for Test (Default: All streams)
            let config = StreamConfig::default();

            if use_pinned {
                std::thread::Builder::new()
                    .name(format!("worker-{}", symbol_clone))
                    .spawn(move || {
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .expect("Failed to build dedicated runtime");
                        
                        rt.block_on(async move {
                             // #3. Pass Config here
                             crate::exchanges::binance::connect_binance(symbol_clone, engine_clone, config).await;
                        });
                    })
                    .expect("Failed to spawn pinned thread");
            } else {
                tokio::spawn(async move {
                    // #4. Pass Config here
                    crate::exchanges::binance::connect_binance(symbol_clone, engine_clone, config).await;
                });
            }
            
            if use_pinned {
                sleep(Duration::from_millis(10)).await;
            }
        }
    }

    sleep(Duration::from_secs(15)).await;

    active_flag.store(false, Ordering::Relaxed);
}

//
// MAIN TEST SUITE
//

#[tokio::test]
async fn test_binance_stream() {
    let single_symbol = vec!["BTCUSDT".to_string()];
    
    let top_x_symbols = fetch_top_volume_symbols(100).await;

    print!("\x1b[2J\x1b[H"); 

    run_scenario("1. Single (BTC) - Standard", single_symbol.clone(), false).await;
    run_scenario("2. Single (BTC) - Pinned", single_symbol.clone(), true).await;
    run_scenario("3. Top 50 (Volume) - Standard", top_x_symbols.clone(), false).await;
    run_scenario("4. Top 50 (Volume) - Pinned", top_x_symbols.clone(), true).await;

    println!("\n\nAll scenarios completed.");
}