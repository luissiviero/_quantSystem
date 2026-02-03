// @file: binance.rs
// @description: Handles WebSocket connections for Binance with exponential backoff reconnection logic.
// @author: v5 helper
// ingestion_engine/src/exchanges/binance.rs

use futures_util::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde::{Deserialize};
use crate::engine::Engine;
use crate::models::{OrderBook, PriceLevel, Trade, TradeSide};
use url::Url;
use std::sync::Arc;
use tokio::time::{sleep, Duration};


//
// CONSTANTS
//


const BINANCE_WS_URL: &str = "wss://stream.binance.com:9443/ws";
const MAX_BACKOFF_SECONDS: u64 = 60;


//
// BINANCE-SPECIFIC WIRE MODELS
//


#[derive(Deserialize)]
struct BinanceTradeEvent {
    #[serde(rename = "t")]
    id: u64,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "p")]
    price: String,
    #[serde(rename = "q")]
    quantity: String,
    #[serde(rename = "T")]
    timestamp: u64,
    #[serde(rename = "m")]
    is_buyer_maker: bool,
}


#[derive(Deserialize)]
struct BinanceDepthEvent {
    #[serde(rename = "lastUpdateId")]
    last_update_id: u64,
    bids: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
}


//
// CONNECTION LOGIC
//


pub async fn connect_binance(symbol: String, engine: Engine) {
    let mut backoff_seconds: u64 = 1;

    // #1. Start infinite reconnection loop
    loop {
        let stream_name: String = format!("{}@depth20/{}@trade", symbol.to_lowercase(), symbol.to_lowercase());
        let url_str: String = format!("{}/{}", BINANCE_WS_URL, stream_name);
        let url: Url = Url::parse(&url_str).expect("Bad URL structure");

        println!("Connecting to Binance: {}", url);

        // #2. Attempt to establish WebSocket connection
        match connect_async(url).await {
            Ok((ws_stream, _)) => {
                println!("Successfully connected to Binance for {}", symbol);
                backoff_seconds = 1;

                let (_, mut read) = ws_stream.split();

                // #3. Process incoming messages
                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            if let Err(e) = handle_message(&symbol, &text, &engine).await {
                                eprintln!("Error handling message: {}", e);
                            }
                        }
                        Ok(Message::Close(frame)) => {
                            eprintln!("Binance closed connection: {:?}", frame);
                            break;
                        }
                        Err(e) => {
                            eprintln!("Websocket error: {}", e);
                            break; 
                        }
                        _ => {} 
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to connect to Binance: {}. Retrying in {}s...", e, backoff_seconds);
            }
        }

        // #4. Exponential Backoff Logic
        sleep(Duration::from_secs(backoff_seconds)).await;
        backoff_seconds = std::cmp::min(backoff_seconds * 2, MAX_BACKOFF_SECONDS);
    }
}


//
// MESSAGE ROUTING & PARSING
//


async fn handle_message(symbol: &str, text: &str, engine: &Engine) -> Result<(), serde_json::Error> {
    // #1. Fast-path check for event type to avoid full generic parse
    if text.contains("\"e\":\"trade\"") {
        let ev: BinanceTradeEvent = serde_json::from_str(text)?;
        
        // #2. Convert to internal Trade model
        let trade: Trade = Trade {
            id: ev.id,
            symbol: ev.symbol,
            price: ev.price.parse::<f64>().unwrap_or(0.0),
            quantity: ev.quantity.parse::<f64>().unwrap_or(0.0),
            timestamp_ms: ev.timestamp,
            side: if ev.is_buyer_maker { TradeSide::Sell } else { TradeSide::Buy },
        };

        engine.add_trade(symbol.to_string(), trade).await;
    } else if text.contains("\"bids\"") {
        let ev: BinanceDepthEvent = serde_json::from_str(text)?;
        
        // #3. Efficient level parsing
        let bids: Vec<PriceLevel> = parse_raw_levels(&ev.bids);
        let asks: Vec<PriceLevel> = parse_raw_levels(&ev.asks);

        let book: OrderBook = OrderBook {
            symbol: symbol.to_string(),
            bids: Arc::new(bids),
            asks: Arc::new(asks),
            last_update_id: ev.last_update_id,
        };

        engine.update_order_book(symbol.to_string(), book).await;
    }

    Ok(())
}


fn parse_raw_levels(raw: &[[String; 2]]) -> Vec<PriceLevel> {
    raw.iter()
        .map(|item| PriceLevel {
            price: item[0].parse::<f64>().unwrap_or(0.0),
            quantity: item[1].parse::<f64>().unwrap_or(0.0),
        })
        .collect()
}