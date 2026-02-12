// @file: ingestion_engine/src/tests/latency.rs
// @description: Optimized Latency Test. Decouples UI rendering from data processing to ensure zero-cost observation.
// @author: LAS.

#[cfg(test)]
mod latency_tests {
    use crate::core::interfaces::DataProcessor;
    use crate::core::models::{MarketData, MarketType}; 
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
        let offset = t1 - estimated_server_time;
        
        println!(">> Clock Sync | RTT: {}ms | Local Offset: {}ms", rtt, offset);
        offset
    }


    //
    // 2. LOCK-FREE SHARED STATS & RESULTS
    //

    struct TestStats {
        trade_count: AtomicU64,
        gap_count: AtomicU64,
        accumulated_jitter: AtomicI64,
        burst_count: AtomicU64,
        last_trade_info: Mutex<String>,
    }

    #[derive(Debug, Clone)]
    struct ScenarioResult {
        title: String,
        avg_latency: f64,
        gaps: u64,
        total_trades: u64,
        tps: f64,
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

    struct LatencyProcessor {
        stats: Arc<TestStats>,
        last_ids: RwLock<HashMap<String, u64>>, 
        last_process_ms: AtomicU64,
        current_burst_depth: AtomicU64,
        clock_offset: i64,
    }

    impl LatencyProcessor {
        fn new(stats: Arc<TestStats>, clock_offset: i64) -> Self {
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
                let adjusted_server_time = (t.timestamp_ms as i64) + self.clock_offset;
                let latency = (now_ms as i64) - adjusted_server_time;

                self.stats.accumulated_jitter.fetch_add(latency, Ordering::Relaxed);
                self.stats.trade_count.fetch_add(1, Ordering::Relaxed);

                // Gap Detection (Requires Lock, but scoped strictly)
                {
                    let mut guard = self.last_ids.write().unwrap();
                    let last_id = guard.entry(t.symbol.clone()).or_insert(0);
                    
                    if *last_id != 0 && t.id != *last_id + 1 {
                        self.stats.gap_count.fetch_add(1, Ordering::Relaxed);
                    }
                    *last_id = t.id;
                }

                // Update UI String (Non-blocking check)
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

    async fn run_scenario(title: &str, symbols: Vec<String>, use_pinned: bool, enable_ui: bool) -> ScenarioResult {
        let offset = get_clock_offset_ms().await;
        let start_time = SystemTime::now();
        
        let stats = Arc::new(TestStats::new());
        let active = Arc::new(AtomicBool::new(true));

        // Config Setup
        let test_config = AppConfig {
            log_level: "info".to_string(),
            default_symbols: vec![],
            broadcast_buffer_size: 100_000, 
            trade_history_limit: 100,
            candle_history_limit: 100,
            
            // Binance Settings
            binance_spot_ws_url: "wss://stream.binance.com:9443/ws".to_string(),
            binance_linear_future_ws_url: "wss://fstream.binance.com/ws".to_string(),
            binance_inverse_future_ws_url: "wss://dstream.binance.com/ws".to_string(),
            
            binance_reconnect_delay: 60,
            order_book_depth: "20".to_string(),
            default_raw_trades: true,
            default_agg_trades: false, // Turn off for cleaner latency test
            default_order_book: false, // Turn off for cleaner latency test
            default_kline_intervals: vec![],
            
            // New Feature Defaults (Initialized to false for Latency Test)
            default_ticker: false,
            default_book_ticker: false,
            default_mark_price: false,
            default_index_price: false,
            default_liquidation: false,
            default_funding_rate: false,
            default_open_interest: false,
            default_greeks: false,

            server_bind_address: "127.0.0.1:0".to_string(),
            server_history_fetch_limit: 100,
        };

        let engine = Engine::new(&test_config);
        
        let processor = Box::new(LatencyProcessor::new(stats.clone(), offset));
        engine.register_processor(processor).await;

        let mon_stats = stats.clone();
        let mon_title = title.to_string();
        let mon_active = active.clone();
        
        if enable_ui {
            tokio::spawn(run_monitor(mon_stats, mon_title, mon_active));
        } else {
            println!("   -> Started background scenario: '{}'", title);
        }

        for symbol in symbols {
            let unique_id: String = format!("BINANCE_SPOT_{}", symbol).to_uppercase();
            
            if engine.request_ingestion(unique_id.clone()).await {
                let engine_clone = engine.clone();
                let sym = symbol.clone();
                let uid = unique_id.clone();
                let cfg = test_config.clone();
                let stream_cfg = cfg.get_stream_config();

                if use_pinned {
                    std::thread::Builder::new().name(format!("w-{}", sym)).spawn(move || {
                        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
                        rt.block_on(async move {
                            crate::connectors::binance::connect_binance(
                                sym, 
                                uid,
                                MarketType::Spot,
                                engine_clone, 
                                stream_cfg, 
                                cfg
                            ).await;
                        });
                    }).unwrap();
                } else {
                    tokio::spawn(async move {
                        crate::connectors::binance::connect_binance(
                            sym,
                            uid,
                            MarketType::Spot,
                            engine_clone, 
                            stream_cfg, 
                            cfg
                        ).await;
                    });
                }
                if use_pinned { sleep(Duration::from_millis(5)).await; }
            }
        }

        sleep(Duration::from_secs(15)).await;
        active.store(false, Ordering::Relaxed);
        sleep(Duration::from_secs(1)).await; 
        
        if enable_ui {
            println!("\n");
        }

        let total_trades = stats.trade_count.load(Ordering::Relaxed);
        let total_jitter = stats.accumulated_jitter.load(Ordering::Relaxed);
        let avg_latency = if total_trades > 0 { total_jitter as f64 / total_trades as f64 } else { 0.0 };
        let gaps = stats.gap_count.load(Ordering::Relaxed);
        let duration_secs = start_time.elapsed().unwrap_or(Duration::from_secs(15)).as_secs_f64();

        ScenarioResult {
            title: title.to_string(),
            avg_latency,
            gaps,
            total_trades,
            tps: total_trades as f64 / duration_secs,
        }
    }

    fn print_comparison(title: &str, standard: &ScenarioResult, pinned: &ScenarioResult) {
        println!("\n=======================================================");
        println!(" COMPARISON: {}", title);
        println!("=======================================================");
        println!(" Metric          | {:<15} | {:<15}", "Standard", "Pinned");
        println!("-----------------|-----------------|-----------------");
        println!(" Avg Latency     | {:<10.2} ms    | {:<10.2} ms", standard.avg_latency, pinned.avg_latency);
        println!(" Gaps Detected   | {:<15} | {:<15}", standard.gaps, pinned.gaps);
        println!(" Trades (Vol)    | {:<15} | {:<15}", standard.total_trades, pinned.total_trades);
        println!(" Est. TPS        | {:<10.2}      | {:<10.2}", standard.tps, pinned.tps);
        println!("=======================================================");
        
        let winner = if pinned.avg_latency < standard.avg_latency { "Pinned" } else { "Standard" };
        let diff = (standard.avg_latency - pinned.avg_latency).abs();
        println!(" >> WINNER: {} (by {:.2} ms)", winner, diff);
        println!("=======================================================\n");
    }


    //
    // MAIN TEST
    //

    #[tokio::test]
    async fn test_latency_suite() {
        // --- CONFIGURATION ---
        const CONCURRENT_MODE: bool = true; 
        let symbol_count = 50; 
        // ---------------------

        let single = vec!["BTCUSDT".to_string()];
        let top_n_symbols = fetch_top_volume_symbols(symbol_count).await;

        print!("\x1b[2J\x1b[H"); 

        let (res_single_std, res_single_pin) = if CONCURRENT_MODE {
            println!("\n>> STARTING CONCURRENT TEST: Single Symbol (BTCUSDT)");
            println!(">> Both engines starting simultaneously (15s duration)...");
            tokio::join!(
                run_scenario("Single (BTC) - Standard", single.clone(), false, false),
                run_scenario("Single (BTC) - Pinned", single.clone(), true, false)
            )
        } else {
            println!("\n>> STARTING SEQUENTIAL TEST: Single Symbol (BTCUSDT)");
            let r1 = run_scenario("Single (BTC) - Standard", single.clone(), false, true).await;
            let r2 = run_scenario("Single (BTC) - Pinned", single.clone(), true, true).await;
            (r1, r2)
        };
        
        let title_std = format!("Top {} - Standard Async", symbol_count);
        let title_pin = format!("Top {} - Pinned Threads", symbol_count);

        let (res_multi_std, res_multi_pin) = if CONCURRENT_MODE {
            println!("\n>> STARTING CONCURRENT TEST: Top {} Symbols", symbol_count);
            println!(">> Both engines starting simultaneously (15s duration)...");
            tokio::join!(
                run_scenario(&title_std, top_n_symbols.clone(), false, false),
                run_scenario(&title_pin, top_n_symbols.clone(), true, false)
            )
        } else {
            println!("\n>> STARTING SEQUENTIAL TEST: Top {} Symbols", symbol_count);
            let r1 = run_scenario(&title_std, top_n_symbols.clone(), false, true).await;
            let r2 = run_scenario(&title_pin, top_n_symbols.clone(), true, true).await;
            (r1, r2)
        };

        // Final Report
        let mode_str = if CONCURRENT_MODE { "(Concurrent Run)" } else { "(Sequential Run)" };
        print_comparison(&format!("Single Symbol {}", mode_str), &res_single_std, &res_single_pin);
        print_comparison(&format!("Top {} Symbols {}", symbol_count, mode_str), &res_multi_std, &res_multi_pin);
    }
}