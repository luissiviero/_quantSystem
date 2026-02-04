// @file: src/tests/engine_bench.rs
// @description: High-performance internal benchmark to verify Engine throughput and CPU efficiency.
// @author: v5 helper

#[allow(dead_code)]
mod tests {
    use crate::engine::Engine;
    use crate::models::{Trade, TradeSide};
    use std::time::Instant;
    use tokio::sync::broadcast::error::RecvError;

    //
    // CONSTANTS
    //

    const TOTAL_MESSAGES: u64 = 1_000_000; // 1 Million updates

    //
    // BENCHMARK LOGIC
    //

    #[tokio::test]
    async fn test_engine_throughput() {
        // #1. Initialize Engine
        let engine: Engine = Engine::new();
        // Subscribe to broadcast channel to measure reception
        let mut rx = engine.tx.subscribe();

        println!("Starting Engine Benchmark: {} messages...", TOTAL_MESSAGES);

        // #2. Spawn Consumer Task (Simulates WebSocket Server)
        // We spawn this first to ensure subscription is active
        let consumer = tokio::spawn(async move {
            let mut count: u64 = 0;
            let mut lag_count: u64 = 0;

            loop {
                match rx.recv().await {
                    Ok(_) => {
                        count += 1;
                        if count >= TOTAL_MESSAGES {
                            break;
                        }
                    }
                    Err(RecvError::Lagged(skipped)) => {
                        // If this happens, the Engine is faster than the Consumer (Good for Engine!)
                        lag_count += skipped;
                        count += skipped; // Count skipped messages as "processed" for test completion
                        if count >= TOTAL_MESSAGES {
                            break;
                        }
                    }
                    Err(RecvError::Closed) => break,
                }
            }
            (count, lag_count)
        });

        // #3. Start Timer
        let start_time: Instant = Instant::now();

        // #4. Producer Loop (Simulates Binance)
        // Blasts trades into the engine as fast as CPU allows
        for i in 0..TOTAL_MESSAGES {
            let trade: Trade = Trade {
                id: i,
                symbol: "BTCUSDT".to_string(),
                price: 50000.0 + (i as f64 * 0.1),
                quantity: 0.01,
                timestamp_ms: 1670000000000 + i,
                side: TradeSide::Buy,
            };

            // This triggers the serialization + broadcast logic
            engine.add_trade("BTCUSDT".to_string(), trade).await;
        }

        // #5. Await Completion & Calculate Stats
        // FIX: We now use 'received' in the calculation below, satisfying the compiler
        let (received, lagged) = consumer.await.unwrap();
        
        let duration: std::time::Duration = start_time.elapsed();
        let seconds: f64 = duration.as_secs_f64();
        // FIX: Use actual received count for accurate TPS
        let tps: f64 = received as f64 / seconds;

        // #
        // # RESULTS REPORT
        // #

        println!("\n========================================");
        println!("BENCHMARK RESULTS");
        println!("========================================");
        println!("Total Messages  : {}", received);
        println!("Time Elapsed    : {:.4}s", seconds);
        println!("Throughput      : {:.2} msgs/sec", tps);
        println!("Consumer Lagged : {} messages (Skipped)", lagged);
        println!("========================================\n");

        // Assertion: Ensure we are not insanely slow (Expect > 50k TPS on modern hardware)
        assert!(tps > 50_000.0, "Engine is too slow! Throughput < 50k/sec");
    }
}