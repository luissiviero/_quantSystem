// @file: binance.rs
// @description: Handles WebSocket connections and parsing for Binance.
// @author: v5 helper

use futures_util::StreamExt; // Fixed: Removed SinkExt warning
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::Value;
use crate::engine::Engine;
use crate::models::{OrderBook, PriceLevel, Trade};
use url::Url;

//
// CONSTANTS
//

const BINANCE_WS_URL: &str = "wss://stream.binance.com:9443/ws";

//
// CONNECTION LOGIC
//

pub async fn connect_binance(symbol: String, engine: Engine) {
    // 1. Format the stream URL (lowercase symbol required by Binance)
    let stream_name: String = format!("{}@depth20/{}@trade", symbol.to_lowercase(), symbol.to_lowercase());
    let url_str: String = format!("{}/{}", BINANCE_WS_URL, stream_name);
    let url: Url = Url::parse(&url_str).expect("Bad URL");

    println!("Connecting to Binance: {}", url);

    // 2. Establish WebSocket connection
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    let (_, mut read) = ws_stream.split();

    // 3. Process incoming messages
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                // 4. Parse JSON
                let v: Value = match serde_json::from_str(&text) {
                    Ok(val) => val,
                    Err(_) => continue,
                };

                // 5. Route based on event type
                if let Some(event_type) = v["e"].as_str() {
                    match event_type {
                        "depthUpdate" => handle_depth_update(&symbol, &v, &engine).await,
                        "trade" => handle_trade(&symbol, &v, &engine).await,
                        _ => {} // Ignore other events
                    }
                } else {
                    // Fallback for direct depth snapshots if not using @depthUpdate
                    if !v["bids"].is_null() {
                         handle_snapshot(&symbol, &v, &engine).await;
                    }
                }
            }
            _ => {} // Ignore non-text messages
        }
    }
}

//
// PARSING LOGIC
//

async fn handle_snapshot(symbol: &str, v: &Value, engine: &Engine) {
    // 1. Parse bids
    let bids: Vec<PriceLevel> = parse_levels(&v["bids"]);
    
    // 2. Parse asks
    let asks: Vec<PriceLevel> = parse_levels(&v["asks"]);
    
    // 3. Create OrderBook struct
    let book: OrderBook = OrderBook {
        symbol: symbol.to_string(),
        bids,
        asks,
        last_update_id: v["lastUpdateId"].as_u64().unwrap_or(0),
    };

    // 4. Update Engine
    engine.update_order_book(symbol.to_string(), book).await;
}

async fn handle_depth_update(symbol: &str, v: &Value, engine: &Engine) {
     // For this simplified implementation, we treat partial depth updates 
     // similar to snapshots as we are using the @depth20 stream.
     handle_snapshot(symbol, v, engine).await;
}

async fn handle_trade(symbol: &str, v: &Value, engine: &Engine) {
    // 1. Extract fields
    let price_str: &str = v["p"].as_str().unwrap_or("0.0");
    let qty_str: &str = v["q"].as_str().unwrap_or("0.0");
    
    // 2. Construct Trade
    let trade: Trade = Trade {
        symbol: symbol.to_string(),
        price: price_str.parse().unwrap_or(0.0),
        quantity: qty_str.parse().unwrap_or(0.0),
        time: v["T"].as_u64().unwrap_or(0),
        is_buyer_maker: v["m"].as_bool().unwrap_or(false),
    };

    // 3. Update Engine
    engine.add_trade(symbol.to_string(), trade).await;
}

fn parse_levels(items: &Value) -> Vec<PriceLevel> {
    // 1. Initialize vector
    let mut levels: Vec<PriceLevel> = Vec::new();
    
    // 2. Iterate and parse
    if let Some(arr) = items.as_array() {
        for item in arr {
            let price_str: &str = item[0].as_str().unwrap_or("0");
            let qty_str: &str = item[1].as_str().unwrap_or("0");
            
            levels.push(PriceLevel {
                price: price_str.parse().unwrap_or(0.0),
                quantity: qty_str.parse().unwrap_or(0.0),
            });
        }
    }
    
    levels
}