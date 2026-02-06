// @file: ingestion_engine/src/tests/latency.rs
// @description: Optimized Latency Test. Decouples UI rendering from data processing to ensure zero-cost observation.
// @author: LAS.

use crate::core::interfaces::DataProcessor;
use crate::core::models::{MarketData, StreamConfig}; 
use crate::core::engine::Engine;
use crate::utils::config::AppConfig;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::time::{sleep, Duration, interval};
use std::sync::{Mutex, RwLock};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use serde::Deserialize;


//
// 1. DYNAMIC SYMBOL FETCHING & CLOCK SYNC
//

#[derive(Deserialize, Debug)]
struct Ticker24hr {
    symbol: String,
    #[serde(rename = "quoteVolume")]
    quote_volume: String, 
}

#[derive(Deserialize, Debug)]
struct ServerTime {
    #[serde(rename = "serverTime")]
    server_time: u64,
}

async fn fetch_top_volume_symbols(limit: usize) -> Vec<String> {
    println!(">> Fetching top {} symbols by volume from Binance API...", limit);
    let client = reqwest::Client::new();
    let resp = client.get("https://api.binance.com/api/v3/ticker/24hr")
        .send().await.expect("Failed to fetch tickers");
    
    let tickers: Vec<Ticker24hr> = resp.json().await.expect("Failed to parse JSON");

    let mut usdt_pairs: Vec<(String, f64)> = tickers.into_iter()
        .filter(|t| t.symbol.ends_with("USDT"))
        .map(|t| (t.symbol, t.quote_volume.parse::<f64>().unwrap_or(0.0)))
        .collect();

    usdt_pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    usdt_pairs.into_iter().take(limit).map(|(s, _)| s).collect()
}

async fn get_clock_offset_ms() -> i64 {
    let client = reqwest::Client::new();
    let t0 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
    
    let resp = client.get("https://api.binance.com/api/v3/time")
        .send().await.expect("Failed to fetch server time");
    
    let t1 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
    let server_data: ServerTime = resp.json().await.expect("Failed to parse server time");
    
    // RTT / 2 is the network delay component
    let rtt = t1 - t0;
    let estimated_server_time = (server_data.server_time as i64) + (rtt / 2);
    
    // Offset = Local - Server
    // If Local is 1000 and Server is 900, Offset is +100.
    // To normalize: Server + Offset = Local
    let offset = t1 - estimated_server_time;
    
    println!(">> Clock Sync | RTT: {}ms | Local Offset: {}ms", rtt, offset);
    offset
}


//
// 2. LOCK-FREE SHARED STATS
//

struct TestStats {
    trade_count: AtomicU64,
    gap_count: AtomicU64,
    accumulated_jitter: AtomicI64,
    burst_count: AtomicU64,
    // We use a mutex only for non-critical, infrequent UI strings
    last_trade_info: Mutex<String>,
}

impl TestStats {
    fn new() -> Self {
        Self {
            trade_count: AtomicU64::new(0),
            gap_count: AtomicU64::new(0),
            accumulated_jitter: AtomicI64::new(0),
            burst_count: AtomicU64::new(0),
            last_trade_info: Mutex::new("Waiting...".to_string()),
        }
    }
}


//
// 3. OPTIMIZED PROCESSOR
//

pub struct LatencyProcessor {
    stats: Arc<TestStats>,
    // Map Symbol -> Last Trade ID. Uses RwLock for better read concurrency,
    // though writes (updates) are frequent.
    // Ideally use dashmap, but RwLock<HashMap> is standard lib.
    last_ids: RwLock<HashMap<String, u64>>, 
    
    // Tracks burst detection per millisecond (Local Time)
    last_process_ms: AtomicU64,
    current_burst_depth: AtomicU64,
    
    clock_offset: i64,
}

impl LatencyProcessor {
    pub fn new(stats: Arc<TestStats>, clock_offset: i64) -> Self {
        Self {
            stats,
            last_ids: RwLock::new(HashMap::new()),
            last_process_ms: AtomicU64::new(0),
            current_burst_depth: AtomicU64::new(0),
            clock_offset,
        }
    }
}

#[async_trait]
impl DataProcessor for LatencyProcessor {
    async fn process(&self, data: Arc<MarketData>) {
        let now_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;

        // --- 1. Burst Detection (Atomic, Lock-Free) ---
        let last = self.last_process_ms.load(Ordering::Relaxed);
        if now_ms == last {
            let depth = self.current_burst_depth.fetch_add(1, Ordering::Relaxed) + 1;
            if depth == 2 {
                self.stats.burst_count.fetch_add(1, Ordering::Relaxed);
            }
        } else {
            self.last_process_ms.store(now_ms, Ordering::Relaxed);
            self.current_burst_depth.store(1, Ordering::Relaxed);
        }

        // --- 2. Trade Processing ---
        if let MarketData::Trade(t) = &*data {
            // Correct Timestamp Drift
            // If Binance says 1000, and our offset is +50 (we are ahead), 
            // Normalized Server Time = 1000 + 50 = 1050.
            // Latency = Local(1055) - Normalized(1050) = 5ms.
            let adjusted_server_time = (t.timestamp_ms as i64) + self.clock_offset;
            let latency = (now_ms as i64) - adjusted_server_time;

            self.stats.accumulated_jitter.fetch_add(latency, Ordering::Relaxed);
            self.stats.trade_count.fetch_add(1, Ordering::Relaxed);

            // Gap Detection (Requires Lock, but scoped strictly)
            {
                // We try read first (optimistic), but since we ALWAYS update,
                // we might as well go straight to write lock for this specific symbol check.
                // However, holding a global write lock on the HashMap is bad for concurrency.
                // A production system would use sharded maps. 
                // For this test, RwLock is "Okay" but represents the main bottleneck.
                let mut guard = self.last_ids.write().unwrap();
                let last_id = guard.entry(t.symbol.clone()).or_insert(0);
                
                if *last_id != 0 && t.id != *last_id + 1 {
                    self.stats.gap_count.fetch_add(1, Ordering::Relaxed);
                }
                *last_id = t.id;
            }

            // Update UI String (Non-blocking check)
            // We only update the string occasionally to avoid mutex contention
            if t.id % 50 == 0 {
                if let Ok(mut str_guard) = self.stats.last_trade_info.try_lock() {
                    *str_guard = format!("{} @ {:.2} [Lat: {}ms]", t.symbol, t.price, latency);
                }
            }
        }
    }

    fn on_error(&self, _error: String) {}
}


//
// 4. BACKGROUND UI MONITOR TASK
//

async fn run_monitor(stats: Arc<TestStats>, title: String, active: Arc<AtomicBool>) {
    let mut lines_printed = 0;
    let mut ticker = interval(Duration::from_millis(250)); // 4 FPS UI update

    while active.load(Ordering::Relaxed) {
        ticker.tick().await;

        let count = stats.trade_count.load(Ordering::Relaxed);
        let gaps = stats.gap_count.load(Ordering::Relaxed);
        let bursts = stats.burst_count.load(Ordering::Relaxed);
        let total_jitter = stats.accumulated_jitter.load(Ordering::Relaxed);
        
        let avg_jitter = if count > 0 {
            total_jitter as f64 / count as f64
        } else { 0.0 };

        let last_info = stats.last_trade_info.lock().unwrap().clone();

        let content = format!(
            "================ STRESS TEST: {} ================\x1b[K\n\
            [Integrity] Trades: {:<8} | Gaps: {:<4} | Bursts: {:<4}\x1b[K\n\
            [Latency]   Avg Latency (adj): {:.2} ms\x1b[K\n\
            [Last Data] {}\x1b[K\n\
            ========================================================================\x1b[K", 
            title, count, gaps, bursts, avg_jitter, last_info
        );

        let new_lines = content.matches('\n').count() + 1;
        if lines_printed > 0 {
            print!("\x1b[{}A", lines_printed);
        }
        println!("{}\r", content);
        lines_printed = new_lines;
    }
}


//
// 5. SCENARIO RUNNER
//

async fn run_scenario(title: &str, symbols: Vec<String>, use_pinned: bool) {
    let offset = get_clock_offset_ms().await;
    
    let stats = Arc::new(TestStats::new());
    let active = Arc::new(AtomicBool::new(true));

    // Config Setup
    let test_config = AppConfig {
        log_level: "info".to_string(),
        default_symbols: vec![],
        broadcast_buffer_size: 100_000, 
        trade_history_limit: 100,
        candle_history_limit: 100,
        binance_ws_url: "wss://stream.binance.com:9443/ws".to_string(),
        binance_reconnect_delay: 60,
        order_book_depth: "20".to_string(),
        default_raw_trades: true,
        default_agg_trades: false, // Turn off for cleaner latency test
        default_order_book: false, // Turn off for cleaner latency test
        default_kline_intervals: vec![],
        server_bind_address: "127.0.0.1:0".to_string(),
        server_history_fetch_limit: 100,
    };

    let engine = Engine::new(&test_config);
    
    // Register the optimized processor
    let processor = Box::new(LatencyProcessor::new(stats.clone(), offset));
    engine.register_processor(processor).await;

    // Spawn Monitor separately
    let mon_stats = stats.clone();
    let mon_title = title.to_string();
    let mon_active = active.clone();
    tokio::spawn(run_monitor(mon_stats, mon_title, mon_active));

    // Spawn Connections
    for symbol in symbols {
        if engine.request_ingestion(symbol.clone()).await {
            let engine_clone = engine.clone();
            let sym = symbol.clone();
            let cfg = test_config.clone();
            let stream_cfg = cfg.get_stream_config();

            if use_pinned {
                std::thread::Builder::new().name(format!("w-{}", sym)).spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
                    rt.block_on(async move {
                         crate::connectors::binance_spot::connect_binance(sym, engine_clone, stream_cfg, cfg).await;
                    });
                }).unwrap();
            } else {
                tokio::spawn(async move {
                    crate::connectors::binance_spot::connect_binance(sym, engine_clone, stream_cfg, cfg).await;
                });
            }
            if use_pinned { sleep(Duration::from_millis(5)).await; }
        }
    }

    sleep(Duration::from_secs(15)).await;
    active.store(false, Ordering::Relaxed);
    sleep(Duration::from_secs(1)).await; // Allow monitor to finish
    println!("\n");
}


//
// MAIN TEST
//

#[tokio::test]
async fn test_latency_suite() {
    let single = vec!["BTCUSDT".to_string()];
    let top_50 = fetch_top_volume_symbols(50).await;

    print!("\x1b[2J\x1b[H"); 
    
    // Warmup
    run_scenario("1. Single (BTC) - Standard", single.clone(), false).await;
    
    // Heavy Load
    run_scenario("2. Top 50 - Standard Async", top_50.clone(), false).await;
    
    // Pinned Thread (Theoretical Best)
    run_scenario("3. Top 50 - Pinned Threads", top_50.clone(), true).await;
}