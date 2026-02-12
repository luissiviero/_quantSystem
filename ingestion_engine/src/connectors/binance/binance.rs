// @file: ingestion_engine/src/connectors/binance.rs
// @description: Binance connector with full feature set (FundingRate via MarkPrice).
// @author: LAS.

use futures_util::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde::Deserialize;
use crate::core::engine::Engine;
use crate::core::models::{
    OrderBook, PriceLevel, Trade, AggTrade, TradeSide, Candle, 
    StreamConfig, MarketType, Ticker, BookTicker, MarkPrice, Liquidation, FundingRate
};
use crate::utils::config::AppConfig;
use url::Url;
use std::sync::Arc;
use tokio::time::{sleep, Duration};


//
// BINANCE WIRE MODELS (Existing)
//

#[derive(Deserialize)]
struct BinanceTradeEvent {
    #[serde(rename = "t")] id: u64,
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
    #[serde(rename = "lastUpdateId")] last_update_id: u64,
    #[serde(rename = "u")] final_update_id: Option<u64>,
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
// NEW WIRE MODELS
//

#[derive(Deserialize)]
struct BinanceTickerEvent {
    #[serde(rename = "p")] price_change: String,
    #[serde(rename = "P")] price_change_percent: String,
    #[serde(rename = "c")] last_price: String,
    #[serde(rename = "o")] open_price: String,
    #[serde(rename = "h")] high_price: String,
    #[serde(rename = "l")] low_price: String,
    #[serde(rename = "v")] volume: String,
    #[serde(rename = "q")] quote_volume: String,
    #[serde(rename = "E")] event_time: u64,
}

#[derive(Deserialize)]
struct BinanceBookTickerEvent {
    #[serde(rename = "b")] best_bid_price: String,
    #[serde(rename = "B")] best_bid_qty: String,
    #[serde(rename = "a")] best_ask_price: String,
    #[serde(rename = "A")] best_ask_qty: String,
}

#[derive(Deserialize)]
struct BinanceMarkPriceEvent {
    #[serde(rename = "p")] mark_price: String,
    #[serde(rename = "i")] index_price: String,
    #[serde(rename = "r")] funding_rate: String, 
    #[serde(rename = "T")] next_funding_time: u64,
}

#[derive(Deserialize)]
struct BinanceLiquidationEvent {
    #[serde(rename = "o")] order: BinanceForceOrder,
}

#[derive(Deserialize)]
struct BinanceForceOrder {
    #[serde(rename = "S")] side: String,
    #[serde(rename = "p")] price: String,
    #[serde(rename = "q")] quantity: String,
}


//
// CONNECTION LOGIC
//

pub async fn connect_binance(
    symbol: String, 
    unique_id: String, 
    market_type: MarketType,
    engine: Engine, 
    raw_config: StreamConfig, 
    app_config: AppConfig 
) {
    let mut backoff_seconds: u64 = 1;

    // #1. SANITIZE CONFIG
    let config = raw_config.sanitize_for_market(market_type);

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
        let mut streams: Vec<String> = Vec::with_capacity(15); 

        // #2. BUILD STREAMS
        if config.order_book {
            streams.push(format!("{}@depth{}", s_lower, app_config.order_book_depth));
        }
        if config.raw_trades {
            streams.push(format!("{}@trade", s_lower));
        }
        if config.agg_trades {
            streams.push(format!("{}@aggTrade", s_lower));
        }
        for interval in &config.kline_intervals {
            streams.push(format!("{}@kline_{}", s_lower, interval));
        }
        
        // New Streams
        if config.ticker {
            streams.push(format!("{}@ticker", s_lower));
        }
        if config.book_ticker {
            streams.push(format!("{}@bookTicker", s_lower));
        }
        if config.mark_price {
            // NOTE: This stream also contains Funding Rate data
            streams.push(format!("{}@markPrice", s_lower));
        }
        if config.liquidation {
            streams.push(format!("{}@forceOrder", s_lower));
        }

        // NOTE: Open Interest stream not explicitly added here as it is not
        // standard across all Binance markets/endpoints. 
        // Engine support is added, but connector logic is skipped to prevent errors.

        if streams.is_empty() {
            eprintln!("Error: No valid streams enabled for {}. Aborting connection.", unique_id);
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

                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            let engine_handle: Engine = engine.clone();
                            let uid: String = unique_id.clone();
                            tokio::spawn(async move {
                                let _ = handle_message(&uid, &text, &engine_handle).await;
                            });
                        }
                        Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
                        Ok(Message::Close(_)) => {
                            println!("Connection closed by server for {}", unique_id);
                            break;
                        }
                        Err(e) => {
                            eprintln!("Error reading message for {}: {}", unique_id, e);
                            break;
                        }
                        _ => {}
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
    
    // 1. Trades
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

    // 2. Depth
    } else if text.contains("\"bids\"") {
        let ev: BinanceDepthEvent = serde_json::from_str(text)?;
        let update_id = ev.final_update_id.unwrap_or(ev.last_update_id);
        engine.update_order_book(unique_id.to_string(), OrderBook {
            symbol: unique_id.to_string(),
            bids: Arc::from(parse_raw_levels(&ev.bids)),
            asks: Arc::from(parse_raw_levels(&ev.asks)),
            last_update_id: update_id,
        }).await;

    // 3. Kline
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

    // 4. AggTrade
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

    // ============================
    // NEW HANDLERS
    // ============================

    // 5. Ticker
    } else if text.contains("\"e\":\"24hrTicker\"") {
        let ev: BinanceTickerEvent = serde_json::from_str(text)?;
        engine.update_ticker(unique_id.to_string(), Ticker {
            symbol: unique_id.to_string(),
            price_change: ev.price_change.parse().unwrap_or(0.0),
            price_change_percent: ev.price_change_percent.parse().unwrap_or(0.0),
            last_price: ev.last_price.parse().unwrap_or(0.0),
            open_price: ev.open_price.parse().unwrap_or(0.0),
            high_price: ev.high_price.parse().unwrap_or(0.0),
            low_price: ev.low_price.parse().unwrap_or(0.0),
            volume: ev.volume.parse().unwrap_or(0.0),
            quote_volume: ev.quote_volume.parse().unwrap_or(0.0),
            timestamp: ev.event_time,
        }).await;

    // 6. BookTicker
    } else if text.contains("\"e\":\"bookTicker\"") {
        let ev: BinanceBookTickerEvent = serde_json::from_str(text)?;
        engine.update_book_ticker(unique_id.to_string(), BookTicker {
            symbol: unique_id.to_string(),
            best_bid_price: ev.best_bid_price.parse().unwrap_or(0.0),
            best_bid_qty: ev.best_bid_qty.parse().unwrap_or(0.0),
            best_ask_price: ev.best_ask_price.parse().unwrap_or(0.0),
            best_ask_qty: ev.best_ask_qty.parse().unwrap_or(0.0),
        }).await;

    // 7. MarkPrice & Funding Rate
    } else if text.contains("\"e\":\"markPriceUpdate\"") {
        let ev: BinanceMarkPriceEvent = serde_json::from_str(text)?;
        
        // Update Mark Price
        engine.update_mark_price(unique_id.to_string(), MarkPrice {
            symbol: unique_id.to_string(),
            mark_price: ev.mark_price.parse().unwrap_or(0.0),
            index_price: ev.index_price.parse().unwrap_or(0.0),
            next_funding_time: ev.next_funding_time,
        }).await;

        // Update Funding Rate (Extracted from same stream)
        engine.update_funding_rate(unique_id.to_string(), FundingRate {
            symbol: unique_id.to_string(),
            rate: ev.funding_rate.parse().unwrap_or(0.0),
            time: ev.next_funding_time,
        }).await;

    // 8. Liquidation
    } else if text.contains("\"e\":\"forceOrder\"") {
        let ev: BinanceLiquidationEvent = serde_json::from_str(text)?;
        let side = match ev.order.side.as_str() {
            "SELL" => TradeSide::Sell,
            _ => TradeSide::Buy,
        };
        
        engine.add_liquidation(unique_id.to_string(), Liquidation {
            symbol: unique_id.to_string(),
            price: ev.order.price.parse().unwrap_or(0.0),
            quantity: ev.order.quantity.parse().unwrap_or(0.0),
            side,
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