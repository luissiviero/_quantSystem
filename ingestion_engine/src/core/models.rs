// @file: ingestion_engine/src/core/models.rs
// @description: Centralized data structures with multi-exchange support.
// @author: LAS.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::fmt;


//
// CONFIGURATION
//

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    pub raw_trades: bool,
    pub agg_trades: bool,
    pub order_book: bool,
    pub kline_intervals: Vec<String>, 
}


//
// EXCHANGE & MARKET TYPES
//

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Exchange {
    Binance,
    Bybit,     // Placeholder for future
    Coinbase,  // Placeholder for future
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MarketType {
    Spot,
    LinearFuture, // USDT-M
    InverseFuture, // COIN-M
    Option,
}

impl fmt::Display for Exchange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Display for MarketType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
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
    pub channel: String, // Symbol
    
    // Multi-Exchange Support
    #[serde(default = "default_exchange")]
    pub exchange: Exchange,
    
    #[serde(default = "default_market")]
    pub market_type: MarketType,
    
    pub end_time: Option<u64>,
    pub config: Option<StreamConfig>, 
}

fn default_exchange() -> Exchange { Exchange::Binance }
fn default_market() -> MarketType { MarketType::Spot }