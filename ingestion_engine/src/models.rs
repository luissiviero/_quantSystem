// @file: models.rs
// @description: Centralized data structures including granular StreamConfig.
// @author: v5 helper
// ingestion_engine/src/models.rs

use serde::{Deserialize, Serialize};
use std::sync::Arc;


//
// CONFIGURATION
//

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    pub raw_trades: bool,
    pub agg_trades: bool,
    pub order_book: bool,
    // Changed from 'klines: bool' to specific intervals
    pub kline_intervals: Vec<String>, 
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            raw_trades: true,
            agg_trades: true,
            order_book: true,
            // Default to common intervals
            kline_intervals: vec![
                "1m".to_string(), 
                "5m".to_string(), 
                "15m".to_string(), 
                "1h".to_string(), 
                "4h".to_string(), 
                "1d".to_string()
            ],
        }
    }
}


//
// ORDER BOOK STRUCTURES
//

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub symbol: String,
    pub bids: Arc<[PriceLevel]>,
    pub asks: Arc<[PriceLevel]>,
    pub last_update_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: f64,
    pub quantity: f64,
}


//
// TRADE STRUCTURES
//

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TradeSide {
    Buy, 
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: u64,
    pub symbol: String,
    pub price: f64,
    pub quantity: f64,
    pub timestamp_ms: u64,
    pub side: TradeSide,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggTrade {
    pub id: u64,
    pub symbol: String,
    pub price: f64,
    pub quantity: f64,
    pub timestamp_ms: u64,
    pub side: TradeSide,
    pub first_trade_id: u64,
    pub last_trade_id: u64,
}


//
// KLINE / CANDLE STRUCTURES
//

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub symbol: String,
    pub interval: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub start_time: u64,
    pub close_time: u64,
    pub is_closed: bool,
}


//
// NETWORKING & COMMANDS
//

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MarketData {
    OrderBook(OrderBook),
    Trade(Trade),
    AggTrade(AggTrade),
    Candle(Candle),
    HistoricalCandles(Vec<Candle>), 
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")] 
pub enum CommandAction {
    Subscribe,
    Unsubscribe,
    FetchHistory, 
}

#[derive(Debug, Deserialize)]
pub struct Command {
    pub action: CommandAction,
    pub channel: String,
    pub end_time: Option<u64>,
    // Optional Config from Frontend
    pub config: Option<StreamConfig>, 
}