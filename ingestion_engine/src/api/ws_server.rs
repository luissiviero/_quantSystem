// @file: ingestion_engine/src/api/ws_server.rs
// @description: WebSocket server with full snapshots including FundingRate and OpenInterest.
// @author: LAS.

use std::net::SocketAddr;
use std::collections::HashSet;
use tokio::net::{TcpListener, TcpStream};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::core::engine::Engine;
use crate::core::models::{Command, CommandAction, MarketData};
use crate::utils::config::AppConfig;

pub async fn start_server(engine: Engine, config: AppConfig) {
    let addr_str = &config.server_bind_address;
    let addr: SocketAddr = addr_str.parse().expect("Invalid address");
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");
    
    println!("WebSocket server listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let engine_clone = engine.clone();
        let config_clone = config.clone();
        tokio::spawn(handle_connection(stream, engine_clone, config_clone));
    }
}

async fn handle_connection(stream: TcpStream, engine: Engine, _config: AppConfig) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("Error during handshake: {}", e);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();
    let mut engine_rx = engine.tx.subscribe();
    let mut subscribed_topics: HashSet<String> = HashSet::new();

    loop {
        tokio::select! {
            // 1. Handle incoming commands from the client
            client_msg = read.next() => {
                match client_msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(cmd) = serde_json::from_str::<Command>(&text) {
                            match cmd.action {
                                CommandAction::Subscribe => {
                                    for topic in cmd.topics {
                                        subscribed_topics.insert(topic.clone());
                                        
                                        // Send initial snapshot if available in Engine state
                                        if let Some(state) = engine.registry.read().await.get(&topic) {
                                            // 1. OrderBook Snapshot
                                            if let Some(book) = &*state.order_book.read().await {
                                                if let Ok(json) = serde_json::to_string(&book) {
                                                    let _ = write.send(Message::Text(json)).await;
                                                }
                                            }
                                            // 2. Recent Trades
                                            {
                                                let trades = state.trades.read().await;
                                                for trade in trades.iter() {
                                                    if let Ok(json) = serde_json::to_string(trade) {
                                                        let _ = write.send(Message::Text(json)).await;
                                                    }
                                                }
                                            }
                                            // 3. Ticker
                                            if let Some(ticker) = &*state.ticker.read().await {
                                                if let Ok(json) = serde_json::to_string(ticker) {
                                                    let _ = write.send(Message::Text(json)).await;
                                                }
                                            }
                                            // 4. BookTicker
                                            if let Some(bt) = &*state.book_ticker.read().await {
                                                if let Ok(json) = serde_json::to_string(bt) {
                                                    let _ = write.send(Message::Text(json)).await;
                                                }
                                            }
                                            // 5. MarkPrice
                                            if let Some(mp) = &*state.mark_price.read().await {
                                                if let Ok(json) = serde_json::to_string(mp) {
                                                    let _ = write.send(Message::Text(json)).await;
                                                }
                                            }
                                            // 6. Liquidations
                                            {
                                                let liqs = state.liquidations.read().await;
                                                for liq in liqs.iter() {
                                                    if let Ok(json) = serde_json::to_string(liq) {
                                                        let _ = write.send(Message::Text(json)).await;
                                                    }
                                                }
                                            }
                                            // 7. FundingRate
                                            if let Some(fr) = &*state.funding_rate.read().await {
                                                if let Ok(json) = serde_json::to_string(fr) {
                                                    let _ = write.send(Message::Text(json)).await;
                                                }
                                            }
                                            // 8. OpenInterest
                                            if let Some(oi) = &*state.open_interest.read().await {
                                                if let Ok(json) = serde_json::to_string(oi) {
                                                    let _ = write.send(Message::Text(json)).await;
                                                }
                                            }
                                        }
                                    }
                                }
                                CommandAction::Unsubscribe => {
                                    for topic in cmd.topics {
                                        subscribed_topics.remove(&topic);
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Err(e)) => {
                        eprintln!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            // 2. Handle broadcast updates from the Engine
            engine_msg = engine_rx.recv() => {
                match engine_msg {
                    Ok((symbol, data_arc)) => {
                        // Check subscription before serializing (optimization)
                        if subscribed_topics.contains(&symbol) {
                            // Serialize the MarketData enum to JSON
                            // Note: MarketData should derive Serialize
                            match serde_json::to_string(&*data_arc) {
                                Ok(json_str) => {
                                    if write.send(Message::Text(json_str)).await.is_err() {
                                        break;
                                    }
                                }
                                Err(e) => eprintln!("Serialization error: {}", e),
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        eprintln!("Client lagged by {} messages - dropping them.", count);
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}