// @file: models.rs
// @description: Centralized data structures for the ingestion engine using Arc and strict types.
// @author: v5 helper
// ingestion_engine/src/models.rs

use serde::{Deserialize, Serialize};
use std::sync::Arc;

//
// ORDER BOOK STRUCTURES
//

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub symbol: String,
    // Using Arc to allow O(1) cloning across threads
    pub bids: Arc<Vec<PriceLevel>>,
    pub asks: Arc<Vec<PriceLevel>>,
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
    Buy,  // Taker was a Buyer (Price UP)
    Sell, // Taker was a Seller (Price DOWN)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: u64,              // Unique Trade ID for deduplication
    pub symbol: String,
    pub price: f64,
    pub quantity: f64,
    pub timestamp_ms: u64,    // Explicit timestamp naming
    pub side: TradeSide,      // Replaces boolean
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

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")] // Automatically handles "subscribe" <-> Subscribe
pub enum CommandAction {
    Subscribe,
    Unsubscribe,
}

#[derive(Debug, Deserialize)]
pub struct Command {
    pub action: CommandAction,
    pub channel: String,
}