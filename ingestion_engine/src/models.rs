// ingestion_engine/src/models.rs
use serde::{Deserialize, Serialize};

// --- DATA FLOW (Output) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MarketData {
    Trade(Trade),
    OrderBook(OrderBook),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub symbol: String,
    pub price: f64,
    pub quantity: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub symbol: String,
    pub bids: Vec<(f64, f64)>, // Price, Qty
    pub asks: Vec<(f64, f64)>,
}

// --- CONTROL FLOW (Input) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
// FIX: Removed rename_all="camelCase" because frontend sends "Trade" (PascalCase)
// If we kept camelCase, Rust would expect "trade".
pub enum DataType {
    Trade,
    Depth5,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum Command {
    #[serde(rename = "subscribe")]
    Subscribe {
        symbol: String,
        #[serde(rename = "dataType")]
        data_type: DataType,
    },
}