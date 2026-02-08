// @file: ingestion_engine/src/connectors/binance.rs
// @description: Generic Binance connector handling Spot and Futures via config.
// @author: LAS.

use futures_util::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde::Deserialize;
use crate::core::engine::Engine;
use crate::core::models::{OrderBook, PriceLevel, Trade, AggTrade, TradeSide, Candle, StreamConfig, MarketType};
use crate::utils::config::AppConfig;
use url::Url;
use std::sync::Arc;
use tokio::time::{sleep, Duration};


//
// BINANCE WIRE MODELS
//

#[derive(Deserialize)]
struct BinanceTradeEvent {
    #[serde(rename = "t")] id: u64,
    // #[serde(rename = "s")] symbol: String, // Not used directly, we use unique_id
    #[serde(rename = "p")] price: String,
    #[serde(rename = "q")] quantity: String,
    #[serde(rename = "T")] timestamp: u64,
    #[serde(rename = "m")] is_buyer_maker: bool,
}

#[derive(Deserialize)]
struct BinanceAggTradeEvent {
    #[serde(rename = "a")] id: u64,
    #[serde(rename = "p")] price: String,
    #[serde(rename = "q")] quantity: String,
    #[serde(rename = "T")] timestamp: u64,
    #[serde(rename = "m")] is_buyer_maker: bool,
    #[serde(rename = "f")] first_trade_id: u64,
    #[serde(rename = "l")] last_trade_id: u64,
}

#[derive(Deserialize)]
struct BinanceDepthEvent {
    #[serde(rename = "lastUpdateId")] last_update_id: u64, // Spot
    #[serde(rename = "u")] final_update_id: Option<u64>, // Futures often use 'u'
    bids: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
}

#[derive(Deserialize)]
struct BinanceKlineEvent {
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

pub async fn connect_binance(
    symbol: String, 
    unique_id: String, // The ID used for storage in Engine
    market_type: MarketType,
    engine: Engine, 
    stream_config: StreamConfig,
    app_config: AppConfig 
) {
    let mut backoff_seconds: u64 = 1;

    // #1. Determine Base URL
    let base_url: String = match market_type {
        MarketType::Spot => app_config.binance_spot_ws_url.clone(),
        MarketType::LinearFuture => app_config.binance_linear_future_ws_url.clone(),
        MarketType::InverseFuture => app_config.binance_inverse_future_ws_url.clone(),
        _ => {
            eprintln!("Unsupported market type for Binance: {:?}", market_type);
            return;
        }
    };

    loop {
        let s_lower: String = symbol.to_lowercase();
        let mut streams: Vec<String> = Vec::with_capacity(10); 

        // #2. Build Stream Names
        if stream_config.order_book {
            streams.push(format!("{}@depth{}", s_lower, app_config.order_book_depth));
        }
        if stream_config.raw_trades {
            streams.push(format!("{}@trade", s_lower));
        }
        if stream_config.agg_trades {
            streams.push(format!("{}@aggTrade", s_lower));
        }

        for interval in &stream_config.kline_intervals {
            streams.push(format!("{}@kline_{}", s_lower, interval));
        }

        if streams.is_empty() {
            eprintln!("Error: No streams enabled for {}. Aborting connection.", unique_id);
            return;
        }

        let url_str: String = format!("{}/{}", base_url, streams.join("/"));
        
        let url = match Url::parse(&url_str) {
            Ok(u) => u,
            Err(e) => {
                eprintln!("URL Parse Error: {}", e);
                return;
            }
        };

        println!("Connecting to {} ({}) via {}", unique_id, market_type, url_str);

        match connect_async(url).await {
            Ok((ws_stream, _)) => {
                backoff_seconds = 1;
                let (_, mut read) = ws_stream.split();

                // FIX: Changed from `while let Some(Ok(Message::Text(text)))` to generic handler
                // This prevents the loop from exiting when a Ping or Pong is received.
                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            let engine_handle: Engine = engine.clone();
                            let uid: String = unique_id.clone();
                            
                            // Spawn processing to avoid blocking read loop
                            tokio::spawn(async move {
                                let _ = handle_message(&uid, &text, &engine_handle).await;
                            });
                        }
                        Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {
                            // Keep connection alive, do not break loop
                        }
                        Ok(Message::Close(_)) => {
                            println!("Connection closed by server for {}", unique_id);
                            break;
                        }
                        Err(e) => {
                            eprintln!("Error reading message for {}: {}", unique_id, e);
                            break;
                        }
                        _ => {} // Ignore binary frames
                    }
                }
            }
            Err(e) => {
                eprintln!("Connection failed for {}: {}", unique_id, e);
                sleep(Duration::from_secs(backoff_seconds)).await;
            }
        }
        
        backoff_seconds = std::cmp::min(backoff_seconds * 2, app_config.binance_reconnect_delay);
    }
}


//
// MESSAGE HANDLER
//

async fn handle_message(unique_id: &str, text: &str, engine: &Engine) -> Result<(), serde_json::Error> {
    
    // #1. Trade Event
    if text.contains("\"e\":\"trade\"") {
        let ev: BinanceTradeEvent = serde_json::from_str(text)?;
        engine.add_trade(unique_id.to_string(), Trade {
            id: ev.id,
            symbol: unique_id.to_string(),
            price: ev.price.parse().unwrap_or(0.0),
            quantity: ev.quantity.parse().unwrap_or(0.0),
            timestamp_ms: ev.timestamp,
            side: if ev.is_buyer_maker { TradeSide::Sell } else { TradeSide::Buy },
        }).await;

    // #2. Order Book (Depth)
    // Note: Futures updates might use "u" for update ID, Spot uses "lastUpdateId".
    } else if text.contains("\"bids\"") {
        let ev: BinanceDepthEvent = serde_json::from_str(text)?;
        
        // Normalize Update ID
        let update_id = ev.final_update_id.unwrap_or(ev.last_update_id);

        engine.update_order_book(unique_id.to_string(), OrderBook {
            symbol: unique_id.to_string(),
            bids: Arc::from(parse_raw_levels(&ev.bids)),
            asks: Arc::from(parse_raw_levels(&ev.asks)),
            last_update_id: update_id,
        }).await;

    // #3. Kline (Candle)
    } else if text.contains("\"e\":\"kline\"") {
        let ev: BinanceKlineEvent = serde_json::from_str(text)?;
        let k = ev.kline;
        engine.add_candle(unique_id.to_string(), Candle {
            symbol: unique_id.to_string(),
            interval: k.interval,
            open: k.open.parse().unwrap_or(0.0),
            high: k.high.parse().unwrap_or(0.0),
            low: k.low.parse().unwrap_or(0.0),
            close: k.close.parse().unwrap_or(0.0),
            volume: k.volume.parse().unwrap_or(0.0),
            start_time: k.start_time,
            close_time: k.close_time,
            is_closed: k.is_closed,
        }).await;

    // #4. Agg Trade
    } else if text.contains("\"e\":\"aggTrade\"") {
        let ev: BinanceAggTradeEvent = serde_json::from_str(text)?;
        engine.add_agg_trade(unique_id.to_string(), AggTrade {
            id: ev.id,
            symbol: unique_id.to_string(),
            price: ev.price.parse().unwrap_or(0.0),
            quantity: ev.quantity.parse().unwrap_or(0.0),
            timestamp_ms: ev.timestamp,
            side: if ev.is_buyer_maker { TradeSide::Sell } else { TradeSide::Buy },
            first_trade_id: ev.first_trade_id,
            last_trade_id: ev.last_trade_id,
        }).await;
    }
    
    Ok(())
}


fn parse_raw_levels(raw: &[[String; 2]]) -> Vec<PriceLevel> {
    raw.iter()
        .map(|item| PriceLevel {
            price: item[0].parse().unwrap_or(0.0),
            quantity: item[1].parse().unwrap_or(0.0),
        })
        .collect()
}