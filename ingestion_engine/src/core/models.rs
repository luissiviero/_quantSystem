// @file: ingestion_engine/src/core/models.rs
// @description: Centralized data structures with added validation logic for market capabilities.
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
    
    // NEW FEATURES
    #[serde(default)] pub ticker: bool,            
    #[serde(default)] pub book_ticker: bool,       
    #[serde(default)] pub mark_price: bool,        
    #[serde(default)] pub index_price: bool,       
    #[serde(default)] pub liquidation: bool,       
    #[serde(default)] pub funding_rate: bool,      
    #[serde(default)] pub open_interest: bool,     
    #[serde(default)] pub greeks: bool,            
}

impl StreamConfig {
    // #1. Centralized Capability Logic
    // This ensures connectors don't need hardcoded "if spot" checks everywhere.
    pub fn sanitize_for_market(&self, market_type: MarketType) -> Self {
        let mut clean = self.clone();

        match market_type {
            MarketType::Spot => {
                // Spot markets do not have these derivative-specific features
                clean.mark_price = false;
                clean.index_price = false;
                clean.liquidation = false;
                clean.funding_rate = false;
                clean.open_interest = false;
                clean.greeks = false;
            },
            MarketType::LinearFuture | MarketType::InverseFuture => {
                // Futures generally don't use "Greeks" (specific to Options)
                clean.greeks = false;
            },
            MarketType::Option => {
                // Options might have it all
            }
        }
        clean
    }
}


//
// EXCHANGE & MARKET TYPES
//

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Exchange {
    Binance,
    Bybit,     
    Coinbase,  
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
// EXISTING STRUCTURES
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
// NEW FEATURE STRUCTURES
//

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticker {
    pub symbol: String,
    pub price_change: f64,
    pub price_change_percent: f64,
    pub last_price: f64,
    pub open_price: f64,
    pub high_price: f64,
    pub low_price: f64,
    pub volume: f64,
    pub quote_volume: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookTicker {
    pub symbol: String,
    pub best_bid_price: f64,
    pub best_bid_qty: f64,
    pub best_ask_price: f64,
    pub best_ask_qty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkPrice {
    pub symbol: String,
    pub mark_price: f64,
    pub index_price: f64,
    pub next_funding_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Liquidation {
    pub symbol: String,
    pub price: f64,
    pub quantity: f64,
    pub side: TradeSide,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingRate {
    pub symbol: String,
    pub rate: f64,
    pub time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenInterest {
    pub symbol: String,
    pub open_interest: f64,
    pub time: u64,
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
    
    // NEW VARIANTS
    Ticker(Ticker),
    BookTicker(BookTicker),
    MarkPrice(MarkPrice),
    Liquidation(Liquidation),
    FundingRate(FundingRate),
    OpenInterest(OpenInterest),
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