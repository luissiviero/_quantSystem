// ingestion_engine/src/exchanges/binance.rs
use crate::interfaces::MarketSource;
use crate::models::{DataType, MarketData, Trade, OrderBook};
use async_trait::async_trait;
use futures_util::{StreamExt, SinkExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::Value;
use url::Url;
use log::{info, error, warn};

pub struct BinanceSource {
    ws_tx: mpsc::UnboundedSender<String>,
}

impl BinanceSource {
    pub fn new() -> Self {
        let (tx, _rx) = mpsc::unbounded_channel(); 
        Self { ws_tx: tx } 
    }
}

#[async_trait]
impl MarketSource for BinanceSource {
    async fn start(&mut self, data_pipe: mpsc::Sender<MarketData>) -> Result<(), String> {
        let url = Url::parse("wss://stream.binance.com:9443/ws").map_err(|e| e.to_string())?;
        info!("Connecting to Binance WebSocket...");

        let (ws_stream, _) = connect_async(url).await.map_err(|e| e.to_string())?;
        info!("Binance Connected Successfully.");

        let (mut write, mut read) = ws_stream.split();

        let (internal_tx, mut internal_rx) = mpsc::unbounded_channel();
        self.ws_tx = internal_tx;

        // Writer Task
        tokio::spawn(async move {
            while let Some(msg) = internal_rx.recv().await {
                // info!("Sending to Binance: {}", msg); 
                if let Err(e) = write.send(Message::Text(msg)).await {
                    error!("Failed to send message to Binance: {}", e);
                }
            }
        });

        // Reader Task
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(v) = serde_json::from_str::<Value>(&text) {
                            if v.get("result").is_some() {
                                info!("Binance confirmed subscription.");
                            }

                            if let Some(event_type) = v.get("e").and_then(|e| e.as_str()) {
                                match event_type {
                                    "aggTrade" => {
                                        match parse_trade(&v) {
                                            Ok(trade) => {
                                                // ENABLED LOG: Confirms data is valid inside Rust
                                                info!("Parsed Trade: {} ${}", trade.symbol, trade.price);
                                                let _ = data_pipe.send(MarketData::Trade(trade)).await;
                                            }
                                            Err(_) => warn!("Failed to parse trade: {}", text),
                                        }
                                    },
                                    _ => {}
                                }
                            }
                            if v.get("bids").is_some() && v.get("asks").is_some() {
                                if let Ok(book) = parse_orderbook(&v) {
                                    let _ = data_pipe.send(MarketData::OrderBook(book)).await;
                                }
                            }
                        }
                    }
                    Ok(Message::Ping(_)) => {}
                    Err(e) => error!("Binance WS Error: {}", e),
                    _ => {}
                }
            }
        });

        Ok(())
    }

    async fn subscribe(&self, symbol: &str, data_type: DataType) -> Result<(), String> {
        let method = match data_type {
            DataType::Trade => "aggTrade",
            DataType::Depth5 => "depth5@100ms",
        };
        
        let params = format!("{}@{}", symbol.to_lowercase(), method);
        
        let payload = serde_json::json!({
            "method": "SUBSCRIBE",
            "params": [params],
            "id": 1
        });

        self.ws_tx.send(payload.to_string())
            .map_err(|_| "Failed to send subscription to internal task".to_string())
    }
}

fn parse_trade(v: &Value) -> Result<Trade, ()> {
    Ok(Trade {
        symbol: v["s"].as_str().ok_or(())?.to_string(),
        price: v["p"].as_str().ok_or(())?.parse().map_err(|_| ())?,
        quantity: v["q"].as_str().ok_or(())?.parse().map_err(|_| ())?,
        timestamp: v["T"].as_u64().ok_or(())?,
    })
}

fn parse_orderbook(v: &Value) -> Result<OrderBook, ()> {
    let parse_level = |arr: &Value| -> (f64, f64) {
        let p = arr[0].as_str().unwrap_or("0").parse().unwrap_or(0.0);
        let q = arr[1].as_str().unwrap_or("0").parse().unwrap_or(0.0);
        (p, q)
    };

    let bids = v["bids"].as_array().unwrap_or(&vec![]).iter().map(parse_level).collect();
    let asks = v["asks"].as_array().unwrap_or(&vec![]).iter().map(parse_level).collect();
    let s = v["s"].as_str().unwrap_or("UNKNOWN").to_string(); 

    Ok(OrderBook { symbol: s, bids, asks })
}