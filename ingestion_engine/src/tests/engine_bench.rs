// @file: ingestion_engine/src/tests/engine_bench.rs
// @description: Internal throughput benchmark to stress test Engine serialization and locking.
// @author: LAS.

#[cfg(test)]
mod throughput_tests {
    use crate::core::engine::Engine;
    use crate::core::models::{Trade, TradeSide};
    use crate::utils::config::AppConfig;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Instant;
    use tokio::sync::broadcast::error::RecvError;

    //
    // CONSTANTS
    //

    const TEST_SYMBOL: &str = "BENCHUSDT";
    const TOTAL_TRADES: usize = 1_000_000;

    //
    // BENCHMARK ENTRY POINT
    //

    #[tokio::test]
    async fn test_engine_throughput() {
        // #1. Setup Configuration
        let config: AppConfig = AppConfig {
            log_level: "error".to_string(),
            default_symbols: vec![TEST_SYMBOL.to_string()],
            broadcast_buffer_size: 100_000, 
            trade_history_limit: 100,
            candle_history_limit: 1000,
            
            // Updated Binance Settings
            binance_spot_ws_url: "wss://stream.binance.com:9443/ws".to_string(),
            binance_linear_future_ws_url: "wss://fstream.binance.com/ws".to_string(),
            binance_inverse_future_ws_url: "wss://dstream.binance.com/ws".to_string(),
            
            binance_reconnect_delay: 5,
            order_book_depth: "20".to_string(),
            default_raw_trades: true,
            default_agg_trades: true,
            default_order_book: true,
            default_kline_intervals: vec!["1m".to_string()],
            
            // New Feature Defaults
            default_ticker: false,
            default_book_ticker: false,
            default_mark_price: false,
            default_index_price: false,
            default_liquidation: false,
            default_funding_rate: false,
            default_open_interest: false,
            default_greeks: false,

            server_bind_address: "127.0.0.1:0".to_string(),
            server_history_fetch_limit: 500,
        };

        let engine: Engine = Engine::new(&config);
        
        // #2. Spawn Consumer (Simulated WebSocket Client)
        let received_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let lagged_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        
        let mut rx = engine.tx.subscribe();
        let rx_received = received_count.clone();
        let rx_lagged = lagged_count.clone();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(_) => {
                        rx_received.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(RecvError::Lagged(skipped)) => {
                        rx_lagged.fetch_add(skipped as usize, Ordering::Relaxed);
                    }
                    Err(RecvError::Closed) => break,
                }
            }
        });

        println!(">> Starting Benchmark: {} Trades on symbol {}", TOTAL_TRADES, TEST_SYMBOL);
        
        // #3. Run Producer (The Benchmark)
        let start_time: Instant = Instant::now();

        for i in 0..TOTAL_TRADES {
            let trade: Trade = Trade {
                id: i as u64,
                symbol: TEST_SYMBOL.to_string(),
                price: 50000.0 + (i as f64 * 0.01),
                quantity: 0.001,
                timestamp_ms: 1670000000000 + i as u64,
                side: if i % 2 == 0 { TradeSide::Buy } else { TradeSide::Sell },
            };

            engine.add_trade(TEST_SYMBOL.to_string(), trade).await;
        }

        let duration = start_time.elapsed();
        
        // #4. Calculate Results
        let seconds: f64 = duration.as_secs_f64();
        let tps: f64 = TOTAL_TRADES as f64 / seconds;
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let total_rx: usize = received_count.load(Ordering::Relaxed);
        let total_lag: usize = lagged_count.load(Ordering::Relaxed);

        //
        // REPORTING
        //

        println!("\n========================================");
        println!("BENCHMARK RESULTS");
        println!("========================================");
        println!("Total Trades Generated : {}", TOTAL_TRADES);
        println!("Time Elapsed           : {:.4} seconds", seconds);
        println!("Throughput (TPS)       : {:.2}", tps);
        println!("----------------------------------------");
        println!("Consumer Metrics (Buffer Stress Test)");
        println!("Messages Received      : {}", total_rx);
        println!("Messages Lagged/Drop   : {}", total_lag);
        println!("========================================\n");

        assert!(tps > 1000.0, "TPS is suspiciously low (< 1k). Check locking logic.");
        assert!(total_rx + total_lag > 0, "Consumer received zero messages. Broken pipe?");
    }
}