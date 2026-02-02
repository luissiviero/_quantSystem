// @file: src/models.rs
// @description: Defines the data structures for all Binance Futures streams.

use serde::{Deserialize, Serialize};

// #
// # COMPOSITE SNAPSHOT (New)
// #
// This will be the "State" we send to the frontend every 200ms.
#[derive(Serialize, Debug, Clone, Default)]
pub struct GlobalSnapshot {
    pub ticker: Option<BookTicker>,
    pub mark_price: Option<MarkPrice>,
    // We send only the *latest* trade/kline in the snapshot, 
    // or we could accumulate them. For simplicity, just the latest.
    pub last_trade: Option<AggTrade>,
    pub last_kline: Option<KlineEvent>,
    pub last_liquidation: Option<ForceOrder>,
}

// #
// # WRAPPER FOR MULTI-STREAM
// #
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CombinedEvent {
    pub stream: String,
    pub data: serde_json::Value, 
}

// #
// # 1. BOOK TICKER
// #
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BookTicker {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "b")]
    pub best_bid: String,
    #[serde(rename = "B")]
    pub bid_qty: String,
    #[serde(rename = "a")]
    pub best_ask: String,
    #[serde(rename = "A")]
    pub ask_qty: String,
}

// #
// # 2. AGGREGATE TRADE
// #
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AggTrade {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "q")]
    pub quantity: String,
    #[serde(rename = "m")]
    pub is_buyer_maker: bool, 
}

// #
// # 3. LIQUIDATION ORDER
// #
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ForceOrder {
    #[serde(rename = "o")]
    pub order_data: ForceOrderData,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ForceOrderData {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "S")]
    pub side: String, 
    #[serde(rename = "q")]
    pub original_quantity: String,
    #[serde(rename = "p")]
    pub price: String,
}

// #
// # 4. MARK PRICE
// #
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MarkPrice {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "r")]
    pub funding_rate: String,
    #[serde(rename = "T")]
    pub next_funding_time: i64,
}

// #
// # 5. KLINE / CANDLESTICK
// #
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct KlineEvent {
    #[serde(rename = "k")]
    pub kline: KlineData,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct KlineData {
    #[serde(rename = "t")]
    pub start_time: i64,
    #[serde(rename = "o")]
    pub open: String,
    #[serde(rename = "c")]
    pub close: String,
    #[serde(rename = "h")]
    pub high: String,
    #[serde(rename = "l")]
    pub low: String,
    #[serde(rename = "v")]
    pub volume: String,
    #[serde(rename = "x")]
    pub is_closed: bool,
}