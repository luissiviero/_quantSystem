// @file: server.rs
// @description: WebSocket server utilizing pre-serialized data streams for O(1) broadcast complexity.
// @author: v5 helper
// ingestion_engine\src\server.rs

use std::net::SocketAddr;
use std::collections::HashSet;
use tokio::net::{TcpListener, TcpStream};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use crate::engine::Engine;
use crate::models::{Command, CommandAction, MarketData, Trade};


//
// CONSTANTS
//


const SERVER_ADDR: &str = "127.0.0.1:8080";


//
// SERVER ENTRY POINT
//


pub async fn start_server(engine: Engine) {
    let addr: SocketAddr = SERVER_ADDR.parse().expect("Invalid address");
    let listener: TcpListener = TcpListener::bind(&addr).await.expect("Failed to bind");
    
    println!("WebSocket server listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let engine_clone: Engine = engine.clone();
        tokio::spawn(handle_connection(stream, engine_clone));
    }
}


//
// CONNECTION HANDLER
//


async fn handle_connection(stream: TcpStream, engine: Engine) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("Error during websocket handshake: {}", e);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();
    let mut engine_rx = engine.tx.subscribe();
    let mut subscribed_topics: HashSet<String> = HashSet::new();

    println!("New client connected");

    // #1. Main Event Loop
    loop {
        tokio::select! {
            // Case A: Client Commands
            client_msg = read.next() => {
                match client_msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(cmd) = serde_json::from_str::<Command>(&text) {
                            match cmd.action {
                                CommandAction::Subscribe => {
                                    let symbol: String = cmd.channel.clone();
                                    subscribed_topics.insert(symbol.clone());

                                    // #1. Send Order Book snapshot
                                    if let Some(book) = engine.get_order_book(&symbol).await {
                                        if let Ok(json) = serde_json::to_string(&MarketData::OrderBook(book)) {
                                            let _ = write.send(Message::Text(json)).await;
                                        }
                                    }

                                    // #2. Send Recent Trades snapshot (Restored Logic)
                                    let trades: Vec<Trade> = engine.get_recent_trades(&symbol).await;
                                    for trade in trades {
                                        if let Ok(json) = serde_json::to_string(&MarketData::Trade(trade)) {
                                            let _ = write.send(Message::Text(json)).await;
                                        }
                                    }
                                }
                                CommandAction::Unsubscribe => {
                                    subscribed_topics.remove(&cmd.channel);
                                }
                            }
                        }
                    }
                    _ => break,
                }
            }

            // Case B: Engine Broadcasts
            engine_msg = engine_rx.recv() => {
                match engine_msg {
                    Ok((json_str, data_arc)) => {
                        // #1. Extract symbol from Arc metadata
                        let symbol: &String = match &*data_arc {
                            MarketData::OrderBook(book) => &book.symbol,
                            MarketData::Trade(trade) => &trade.symbol,
                        };

                        // #2. Direct string forward (Zero-compute egress)
                        if subscribed_topics.contains(symbol) {
                            if write.send(Message::Text(json_str)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        continue; // Client too slow, skip frames but keep connection
                    }
                    Err(_) => break,
                }
            }
        }
    }
    
    println!("Client disconnected");
}