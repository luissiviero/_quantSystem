// @file: ingestion_engine/src/connectors/binance_spot.rs
// @description: Binance connector with granular stream subscription and AggTrade parsing restored.
// @author: LAS.

use futures_util::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde::{Deserialize};
use crate::core::engine::Engine;
use crate::core::models::{OrderBook, PriceLevel, Trade, AggTrade, TradeSide, Candle, StreamConfig};
use crate::utils::config::AppConfig;
use url::Url;
use std::sync::Arc;
use tokio::time::{sleep, Duration};


//
// BINANCE-SPECIFIC WIRE MODELS
//

#[derive(Deserialize)]
struct BinanceTradeEvent {
    #[serde(rename = "t")] id: u64,
    #[serde(rename = "s")] symbol: String,
    #[serde(rename = "p")] price: String,
    #[serde(rename = "q")] quantity: String,
    #[serde(rename = "T")] timestamp: u64,
    #[serde(rename = "m")] is_buyer_maker: bool,
}

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

#[derive(Deserialize)]
struct BinanceDepthEvent {
    #[serde(rename = "lastUpdateId")] last_update_id: u64,
    bids: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
}

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

pub async fn connect_binance(
    symbol: String, 
    engine: Engine, 
    stream_config: StreamConfig,
    app_config: AppConfig // #1. Receives dynamic config
) {
    let mut backoff_seconds: u64 = 1;

    loop {
        let s_lower = symbol.to_lowercase();
        let mut streams = Vec::with_capacity(10); 

        if stream_config.order_book {
            // #2. Dynamic Depth (e.g., "depth5", "depth20")
            streams.push(format!("{}@depth{}", s_lower, app_config.order_book_depth));
        }
        if stream_config.raw_trades {
            streams.push(format!("{}@trade", s_lower));
        }
        if stream_config.agg_trades {
            streams.push(format!("{}@aggTrade", s_lower));
        }

        // Dynamic Interval Subscription
        for interval in &stream_config.kline_intervals {
            streams.push(format!("{}@kline_{}", s_lower, interval));
        }

        if streams.is_empty() {
            eprintln!("Error: No streams enabled for {}. Aborting connection.", symbol);
            return;
        }

        // #3. Dynamic URL Construction
        let url_str = format!("{}/{}", app_config.binance_ws_url, streams.join("/"));
        let url = Url::parse(&url_str).unwrap();

        match connect_async(url).await {
            Ok((ws_stream, _)) => {
                // Reset backoff on successful connection
                backoff_seconds = 1;
                let (_, mut read) = ws_stream.split();

                while let Some(Ok(Message::Text(text))) = read.next().await {
                    let engine_handle = engine.clone();
                    let sym = symbol.clone();
                    tokio::spawn(async move {
                        let _ = handle_message(&sym, &text, &engine_handle).await;
                    });
                }
            }
            Err(_) => sleep(Duration::from_secs(backoff_seconds)).await,
        }
        
        // #4. Dynamic Reconnect Delay Cap
        backoff_seconds = std::cmp::min(backoff_seconds * 2, app_config.binance_reconnect_delay);
    }
}


//
// MESSAGE HANDLER
//

async fn handle_message(symbol: &str, text: &str, engine: &Engine) -> Result<(), serde_json::Error> {
    if text.contains("\"e\":\"trade\"") {
        let ev: BinanceTradeEvent = serde_json::from_str(text)?;
        engine.add_trade(symbol.to_string(), Trade {
            id: ev.id,
            symbol: ev.symbol,
            price: ev.price.parse().unwrap_or(0.0),
            quantity: ev.quantity.parse().unwrap_or(0.0),
            timestamp_ms: ev.timestamp,
            side: if ev.is_buyer_maker { TradeSide::Sell } else { TradeSide::Buy },
        }).await;

    } else if text.contains("\"bids\"") {
        let ev: BinanceDepthEvent = serde_json::from_str(text)?;
        engine.update_order_book(symbol.to_string(), OrderBook {
            symbol: symbol.to_string(),
            bids: Arc::from(parse_raw_levels(&ev.bids)),
            asks: Arc::from(parse_raw_levels(&ev.asks)),
            last_update_id: ev.last_update_id,
        }).await;

    } else if text.contains("\"e\":\"kline\"") {
        let ev: BinanceKlineEvent = serde_json::from_str(text)?;
        let k = ev.kline;
        engine.add_candle(symbol.to_string(), Candle {
            symbol: ev.symbol,
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

    } else if text.contains("\"e\":\"aggTrade\"") {
        let ev: BinanceAggTradeEvent = serde_json::from_str(text)?;
        engine.add_agg_trade(symbol.to_string(), AggTrade {
            id: ev.id,
            symbol: ev.symbol,
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