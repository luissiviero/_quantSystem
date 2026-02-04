// @file: binance.rs
// @description: Handles WebSocket connections for Binance with generic interval parsing.
// @author: v5 helper
// ingestion_engine/src/exchanges/binance.rs

use futures_util::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde::{Deserialize};
use crate::engine::Engine;
use crate::models::{OrderBook, PriceLevel, Trade, AggTrade, TradeSide, Candle};
use url::Url;
use std::sync::Arc;
use tokio::time::{sleep, Duration};


//
// CONSTANTS
//

const BINANCE_WS_URL: &str = "wss://stream.binance.com:9443/ws";
const MAX_BACKOFF_SECONDS: u64 = 60;
const KLINE_INTERVALS: [&str; 6] = ["1m", "5m", "15m", "1h", "4h", "1d"]; 


//
// BINANCE-SPECIFIC WIRE MODELS
//

// last trades
#[derive(Deserialize)]
struct BinanceTradeEvent {
    #[serde(rename = "t")] id: u64,
    #[serde(rename = "s")] symbol: String,
    #[serde(rename = "p")] price: String,
    #[serde(rename = "q")] quantity: String,
    #[serde(rename = "T")] timestamp: u64,
    #[serde(rename = "m")] is_buyer_maker: bool,
}

// agg trades
#[derive(Deserialize)]
struct BinanceAggTradeEvent {
    #[serde(rename = "a")] id: u64,
    #[serde(rename = "s")] symbol: String,
    #[serde(rename = "p")] price: String,
    #[serde(rename = "q")] quantity: String,
    #[serde(rename = "T")] timestamp: u64,
    #[serde(rename = "m")] is_buyer_maker: bool,
    #[serde(rename = "f")] first_trade_id: u64,
    #[serde(rename = "l")] last_trade_id: u64,
}

// order book
#[derive(Deserialize)]
struct BinanceDepthEvent {
    #[serde(rename = "lastUpdateId")] last_update_id: u64,
    bids: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
}

// klines
#[derive(Deserialize)]
struct BinanceKlineEvent {
    #[serde(rename = "s")] symbol: String,
    #[serde(rename = "k")] kline: BinanceKlineData,
}

#[derive(Deserialize)]
struct BinanceKlineData {
    #[serde(rename = "t")] start_time: u64,
    #[serde(rename = "T")] close_time: u64,
    #[serde(rename = "o")] open: String,
    #[serde(rename = "c")] close: String,
    #[serde(rename = "h")] high: String,
    #[serde(rename = "l")] low: String,
    #[serde(rename = "v")] volume: String,
    #[serde(rename = "x")] is_closed: bool,
    #[serde(rename = "i")] interval: String,
}


//
// CONNECTION LOGIC
//

pub async fn connect_binance(symbol: String, engine: Engine) {
    let mut backoff_seconds: u64 = 1;

    // #1. Start infinite reconnection loop
    loop {
        let s_lower: String = symbol.to_lowercase();
        
        // #2. Dynamic URL construction
        let mut streams: Vec<String> = vec![
            format!("{}@depth20", s_lower),
            format!("{}@trade", s_lower),
            format!("{}@aggTrade", s_lower), // Added aggTrade stream
        ];

        // Automatically add all configured intervals
        for interval in KLINE_INTERVALS {
            streams.push(format!("{}@kline_{}", s_lower, interval));
        }

        let stream_string: String = streams.join("/");
        let url_str: String = format!("{}/{}", BINANCE_WS_URL, stream_string);
        let url: Url = Url::parse(&url_str).expect("Bad URL structure");

        // FIX: Conditional compilation to hide logs during 'cargo test'
        #[cfg(not(test))]
        println!("Connecting to Binance: {}", url);

        match connect_async(url).await {
            Ok((ws_stream, _)) => {
                // FIX: Conditional compilation to hide logs during 'cargo test'
                #[cfg(not(test))]
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
    if text.contains("\"e\":\"trade\"") {
        let ev: BinanceTradeEvent = serde_json::from_str(text)?;
        let trade: Trade = Trade {
            id: ev.id,
            symbol: ev.symbol,
            price: ev.price.parse::<f64>().unwrap_or(0.0),
            quantity: ev.quantity.parse::<f64>().unwrap_or(0.0),
            timestamp_ms: ev.timestamp,
            side: if ev.is_buyer_maker { TradeSide::Sell } else { TradeSide::Buy },
        };
        engine.add_trade(symbol.to_string(), trade).await;

    } else if text.contains("\"e\":\"aggTrade\"") {
        let ev: BinanceAggTradeEvent = serde_json::from_str(text)?;
        let agg_trade: AggTrade = AggTrade {
            id: ev.id,
            symbol: ev.symbol,
            price: ev.price.parse::<f64>().unwrap_or(0.0),
            quantity: ev.quantity.parse::<f64>().unwrap_or(0.0),
            timestamp_ms: ev.timestamp,
            side: if ev.is_buyer_maker { TradeSide::Sell } else { TradeSide::Buy },
            first_trade_id: ev.first_trade_id,
            last_trade_id: ev.last_trade_id,
        };
        engine.add_agg_trade(symbol.to_string(), agg_trade).await;

    } else if text.contains("\"bids\"") {
        let ev: BinanceDepthEvent = serde_json::from_str(text)?;
        let bids: Vec<PriceLevel> = parse_raw_levels(&ev.bids);
        let asks: Vec<PriceLevel> = parse_raw_levels(&ev.asks);
        let book: OrderBook = OrderBook {
            symbol: symbol.to_string(),
            bids: Arc::from(bids),
            asks: Arc::from(asks),
            last_update_id: ev.last_update_id,
        };
        engine.update_order_book(symbol.to_string(), book).await;

    } else if text.contains("\"e\":\"kline\"") {
        let ev: BinanceKlineEvent = serde_json::from_str(text)?;
        let k: BinanceKlineData = ev.kline;
        
        // #3. Generic Construction
        let candle: Candle = Candle {
            symbol: ev.symbol,
            interval: k.interval,
            open: k.open.parse::<f64>().unwrap_or(0.0),
            high: k.high.parse::<f64>().unwrap_or(0.0),
            low: k.low.parse::<f64>().unwrap_or(0.0),
            close: k.close.parse::<f64>().unwrap_or(0.0),
            volume: k.volume.parse::<f64>().unwrap_or(0.0),
            start_time: k.start_time,
            close_time: k.close_time,
            is_closed: k.is_closed,
        };
        engine.add_candle(symbol.to_string(), candle).await;
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