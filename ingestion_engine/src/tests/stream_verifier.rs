// @file: ingestion_engine/src/tests/stream_verifier.rs
// @description: Integration test ensuring StreamConfig logic correctly filters data streams with new fields.
// @author: LAS.

#[cfg(test)]
mod stream_verification_tests {
    use crate::core::engine::Engine;
    use crate::core::models::{StreamConfig, Trade, OrderBook, TradeSide, PriceLevel, MarketData};
    use crate::utils::config::AppConfig;
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};

    //
    // MOCK CONNECTOR SIMULATION
    //
    async fn mock_connector_loop(symbol: String, engine: Engine, config: StreamConfig) {
        // 1. Simulate Order Book Update
        if config.order_book {
            let book = OrderBook {
                symbol: symbol.clone(),
                bids: Arc::from(vec![PriceLevel { price: 100.0, quantity: 1.0 }]),
                asks: Arc::from(vec![PriceLevel { price: 101.0, quantity: 1.0 }]),
                last_update_id: 12345,
            };
            engine.update_order_book(symbol.clone(), book).await;
        }

        // 2. Simulate Raw Trade
        if config.raw_trades {
            let trade = Trade {
                id: 1,
                symbol: symbol.clone(),
                price: 100.5,
                quantity: 0.1,
                timestamp_ms: 1670000000000,
                side: TradeSide::Buy,
            };
            engine.add_trade(symbol.clone(), trade).await;
        }
    }

    //
    // TEST: RAW TRADES ONLY (The "Firehose" Check)
    //
    #[tokio::test]
    async fn test_verify_raw_trades_only_filter() {
        // #1. Setup Engine with minimal config
        let app_config = AppConfig {
            log_level: "error".to_string(),
            default_symbols: vec![],
            broadcast_buffer_size: 100,
            trade_history_limit: 10,
            candle_history_limit: 10,
            
            // Updated Settings
            binance_spot_ws_url: "".to_string(),
            binance_linear_future_ws_url: "".to_string(),
            binance_inverse_future_ws_url: "".to_string(),
            
            binance_reconnect_delay: 0,
            order_book_depth: "5".to_string(),
            default_raw_trades: true,
            default_agg_trades: true,
            default_order_book: true,
            default_kline_intervals: vec![],
            
            // New Defaults
            default_ticker: false,
            default_book_ticker: false,
            default_mark_price: false,
            default_index_price: false,
            default_liquidation: false,
            default_funding_rate: false,
            default_open_interest: false,
            default_greeks: false,

            server_bind_address: "127.0.0.1:0".to_string(),
            server_history_fetch_limit: 10,
        };
        let engine = Engine::new(&app_config);
        let mut rx = engine.tx.subscribe();

        // #2. Define "Safe" Configuration (Trades Only, NO OrderBook)
        let safe_config = StreamConfig {
            raw_trades: true,
            agg_trades: false,
            order_book: false, // <--- CRITICAL: We are disabling the heavy stream
            kline_intervals: vec![],
            // New Fields
            ticker: false,
            book_ticker: false,
            mark_price: false,
            index_price: false,
            liquidation: false,
            funding_rate: false,
            open_interest: false,
            greeks: false,
        };

        let symbol = "BTCUSDT".to_string();

        println!(">> Test Started: Verifying 'Raw Trades Only' does not leak OrderBooks...");

        // #3. Run the Mock Connector
        mock_connector_loop(symbol.clone(), engine.clone(), safe_config).await;

        // #4. Verify Output
        let mut received_trade = false;
        let mut received_ob = false;

        let listen_duration = Duration::from_millis(100);
        let start = tokio::time::Instant::now();

        while start.elapsed() < listen_duration {
            if let Ok(Ok((_, data))) = timeout(Duration::from_millis(10), rx.recv()).await {
                match *data {
                    MarketData::Trade(_) => received_trade = true,
                    MarketData::OrderBook(_) => received_ob = true,
                    _ => {}
                }
            }
        }

        assert!(received_trade, "CRITICAL: Engine failed to broadcast requested Trade data.");
        assert!(!received_ob, "FATAL: Data Leak! Engine broadcasted OrderBook data despite 'order_book: false'.");
        println!(">> SUCCESS: System respected the filter. Only Trades received.");
    }

    //
    // TEST: ORDER BOOK ONLY
    //
    #[tokio::test]
    async fn test_verify_order_book_only_filter() {
        let app_config = AppConfig {
            log_level: "error".to_string(),
            default_symbols: vec![],
            broadcast_buffer_size: 100,
            trade_history_limit: 10,
            candle_history_limit: 10,
            binance_spot_ws_url: "".to_string(),
            binance_linear_future_ws_url: "".to_string(),
            binance_inverse_future_ws_url: "".to_string(),
            binance_reconnect_delay: 0,
            order_book_depth: "5".to_string(),
            default_raw_trades: true,
            default_agg_trades: true,
            default_order_book: true,
            default_kline_intervals: vec![],
            
            // New Defaults
            default_ticker: false,
            default_book_ticker: false,
            default_mark_price: false,
            default_index_price: false,
            default_liquidation: false,
            default_funding_rate: false,
            default_open_interest: false,
            default_greeks: false,
            
            server_bind_address: "127.0.0.1:0".to_string(),
            server_history_fetch_limit: 10,
        };
        let engine = Engine::new(&app_config);
        let mut rx = engine.tx.subscribe();

        let heavy_config = StreamConfig {
            raw_trades: false, // <--- Disabled
            agg_trades: false,
            order_book: true,  // <--- Enabled
            kline_intervals: vec![],
            // New Fields
            ticker: false,
            book_ticker: false,
            mark_price: false,
            index_price: false,
            liquidation: false,
            funding_rate: false,
            open_interest: false,
            greeks: false,
        };

        let symbol = "ETHUSDT".to_string();

        mock_connector_loop(symbol.clone(), engine.clone(), heavy_config).await;

        let mut received_trade = false;
        let mut received_ob = false;

        while let Ok(Ok((_, data))) = timeout(Duration::from_millis(50), rx.recv()).await {
             match *data {
                MarketData::Trade(_) => received_trade = true,
                MarketData::OrderBook(_) => received_ob = true,
                _ => {}
            }
        }

        assert!(received_ob, "CRITICAL: Engine failed to broadcast requested OrderBook.");
        assert!(!received_trade, "FATAL: Data Leak! Engine broadcasted Trade data despite 'raw_trades: false'.");
        
        println!(">> SUCCESS: System respected the filter. Only OrderBook received.");
    }

    //
    // TEST: DEFAULT CONFIG MAPPING
    //
    #[tokio::test]
    async fn test_app_config_defaults() {
        let app_config = AppConfig {
            log_level: "info".to_string(),
            default_symbols: vec![],
            broadcast_buffer_size: 100,
            trade_history_limit: 10,
            candle_history_limit: 10,
            binance_spot_ws_url: "".to_string(),
            binance_linear_future_ws_url: "".to_string(),
            binance_inverse_future_ws_url: "".to_string(),
            binance_reconnect_delay: 0,
            order_book_depth: "20".to_string(),
            default_raw_trades: true,
            default_agg_trades: false,
            default_order_book: true,
            default_kline_intervals: vec!["1h".to_string()],
            
            // New Defaults
            default_ticker: false,
            default_book_ticker: false,
            default_mark_price: false,
            default_index_price: false,
            default_liquidation: false,
            default_funding_rate: false,
            default_open_interest: false,
            default_greeks: false,
            
            server_bind_address: "127.0.0.1:0".to_string(),
            server_history_fetch_limit: 10,
        };

        let stream_config = app_config.get_stream_config();

        assert_eq!(stream_config.raw_trades, true);
        assert_eq!(stream_config.agg_trades, false); 
        assert_eq!(stream_config.order_book, true);
        assert_eq!(stream_config.kline_intervals.len(), 1);
        assert_eq!(stream_config.kline_intervals[0], "1h");
        
        // Assert New Defaults
        assert_eq!(stream_config.ticker, false);
        assert_eq!(stream_config.book_ticker, false);
        
        println!(">> SUCCESS: AppConfig correctly maps to StreamConfig defaults.");
    }
}