// @file: src/ingestion.rs
// @description: Handles the WebSocket connection to Binance and data parsing for all timeframes.

use crate::models::{AggTrade, BookTicker, CombinedEvent, ForceOrder, MarkPrice, KlineEvent, GlobalSnapshot};
use futures_util::StreamExt;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

pub async fn start_ingestion(shared_state: Arc<RwLock<GlobalSnapshot>>) {
    // #
    // # 1. CONSTRUCT URL
    // #
    let base = "wss://fstream.binance.com/stream?streams=";
    let streams = vec![
        "btcusdt@bookTicker",
        "btcusdt@aggTrade",
        "btcusdt@forceOrder",
        "btcusdt@markPrice",
        "btcusdt@kline_1m",
        "btcusdt@kline_5m",
        "btcusdt@kline_15m",
        "btcusdt@kline_1h",
        "btcusdt@kline_1d",
    ];
    let url_str = format!("{}{}", base, streams.join("/"));
    let url = Url::parse(&url_str).unwrap();

    println!("-----------------------------------------");
    println!("Connecting to Binance Combined Stream...");
    println!("Streams: {:?}", streams);
    println!("-----------------------------------------");

    loop {
        match connect_async(url.to_string()).await {
            Ok((ws_stream, _)) => {
                println!("✅ Connected to Binance!");
                let (_, mut read) = ws_stream.split();

                while let Some(message) = read.next().await {
                    if let Ok(Message::Text(text)) = message {
                        if let Ok(wrapper) = serde_json::from_str::<CombinedEvent>(&text) {
                            
                            // #
                            // # UPDATE SHARED STATE
                            // #
                            let mut lock = shared_state.write().await;

                            match wrapper.stream.as_str() {
                                s if s.contains("@aggTrade") => {
                                    if let Ok(trade) = serde_json::from_value::<AggTrade>(wrapper.data) {
                                        lock.last_trade = Some(trade);
                                    }
                                }
                                s if s.contains("@forceOrder") => {
                                    if let Ok(liq) = serde_json::from_value::<ForceOrder>(wrapper.data) {
                                        lock.last_liquidation = Some(liq);
                                    }
                                }
                                s if s.contains("@markPrice") => {
                                    if let Ok(mp) = serde_json::from_value::<MarkPrice>(wrapper.data) {
                                        lock.mark_price = Some(mp);
                                    }
                                }
                                s if s.contains("@bookTicker") => {
                                    if let Ok(bt) = serde_json::from_value::<BookTicker>(wrapper.data) {
                                        lock.ticker = Some(bt);
                                    }
                                }
                                s if s.contains("@kline") => {
                                    if let Ok(k) = serde_json::from_value::<KlineEvent>(wrapper.data) {
                                        lock.last_kline = Some(k);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                println!("❌ Disconnected. Reconnecting...");
            }
            Err(e) => {
                eprintln!("Connection Error: {}. Retrying in 5s...", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }
}