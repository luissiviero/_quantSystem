    // @file: ingestion_engine/src/connectors/binance_rest.rs
    // @description: HTTP Client for fetching historical klines from Binance.
    // @author: V5 Helper.

    use reqwest::Client;
    use serde_json::Value;
    use crate::core::models::{Candle, MarketType};

    //
    // PUBLIC INTERFACE
    //

    pub async fn fetch_binance_history(
        symbol: &str,
        market: MarketType,
        interval: &str,
        limit: usize
    ) -> Result<Vec<Candle>, String> {
        // #1. Determine API Endpoint
        // Select the correct base URL based on the market type (Spot vs Futures)
        let base_url: &str = match market {
            MarketType::Spot => "https://api.binance.com",
            MarketType::LinearFuture => "https://fapi.binance.com",
            MarketType::InverseFuture => "https://dapi.binance.com",
            _ => return Err(format!("Unsupported market type for REST: {:?}", market)),
        };

        // #2. Construct URL
        // Format: /api/v3/klines?symbol=BTCUSDT&interval=1m&limit=1000
        // Note: Futures use /fapi/v1/klines or /dapi/v1/klines
        let endpoint: &str = match market {
            MarketType::Spot => "/api/v3/klines",
            MarketType::LinearFuture => "/fapi/v1/klines",
            MarketType::InverseFuture => "/dapi/v1/klines",
            _ => "/api/v3/klines",
        };

        let url: String = format!(
            "{}{}?symbol={}&interval={}&limit={}",
            base_url, endpoint, symbol.to_uppercase(), interval, limit
        );

        // #3. Execute Request
        let client: Client = Client::new();
        let response = client.get(&url).send().await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API Error: {}", response.status()));
        }

        let json: Value = response.json().await
            .map_err(|e| format!("JSON Parse Error: {}", e))?;

        // #4. Parse Response
        // Binance returns an array of arrays:
        // [ [Open Time, Open, High, Low, Close, Volume, Close Time, ...], ... ]
        parse_kline_array(symbol, interval, json)
    }

    //
    // INTERNAL HELPERS
    //

    fn parse_kline_array(symbol: &str, interval: &str, json: Value) -> Result<Vec<Candle>, String> {
        let raw_list = json.as_array()
            .ok_or("Invalid response format: Expected array")?;

        let mut candles: Vec<Candle> = Vec::with_capacity(raw_list.len());

        // #1. Iterate and Map
        for item in raw_list {
            let arr = item.as_array().ok_or("Invalid candle format")?;
            
            if arr.len() < 7 {
                continue; // Skip malformed entries
            }

            // #2. Extract Fields safely
            // Helper closure to extract f64 from string or number
            let get_f64 = |idx: usize| -> f64 {
                if let Some(v) = arr.get(idx) {
                    if let Some(s) = v.as_str() {
                        return s.parse().unwrap_or(0.0);
                    }
                }
                0.0
            };

            let get_u64 = |idx: usize| -> u64 {
                arr.get(idx).and_then(|v| v.as_u64()).unwrap_or(0)
            };

            // #3. Construct Candle
            let candle = Candle {
                symbol: symbol.to_string(),
                interval: interval.to_string(),
                start_time: get_u64(0),
                open: get_f64(1),
                high: get_f64(2),
                low: get_f64(3),
                close: get_f64(4),
                volume: get_f64(5),
                close_time: get_u64(6),
                is_closed: true, // Historical candles are always closed
            };

            candles.push(candle);
        }

        Ok(candles)
    }