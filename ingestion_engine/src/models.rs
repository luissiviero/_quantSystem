// @file: models.rs
// @description: Centralized data structures for the ingestion engine.
// @author: v5 helper

use serde::{Deserialize, Serialize};

//
// ORDER BOOK STRUCTURES
//

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub symbol: String,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub symbol: String,
    pub price: f64,
    pub quantity: f64,
    pub time: u64,
    pub is_buyer_maker: bool,
}

//
// NETWORKING & COMMANDS
//

// FIX: Added Clone trait derived here to satisfy broadcast channel requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MarketData {
    OrderBook(OrderBook),
    Trade(Trade),
}

#[derive(Debug, Deserialize)]
pub struct Command {
    pub action: String, // e.g., "subscribe"
    pub channel: String, // e.g., "BTCUSDT"
}