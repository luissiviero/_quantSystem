// @file: ingestion_engine/src/api/ws_server.rs
// @description: WebSocket server handling client commands and config extraction.
// @author: LAS.

use std::net::SocketAddr;
use std::collections::HashSet;
use tokio::net::{TcpListener, TcpStream};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use crate::core::engine::Engine;
use crate::core::models::{Command, CommandAction, MarketData}; 
use crate::connectors::binance_spot; 
use crate::utils::config::AppConfig;


pub async fn start_server(engine: Engine, config: AppConfig) {
    // #1. Use configured bind address
    let addr: SocketAddr = config.server_bind_address.parse().expect("Invalid address");
    let listener: TcpListener = TcpListener::bind(&addr).await.expect("Failed to bind");
    
    println!("WebSocket server listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let engine_clone: Engine = engine.clone();
        let config_clone: AppConfig = config.clone();
        tokio::spawn(handle_connection(stream, engine_clone, config_clone));
    }
}


async fn handle_connection(stream: TcpStream, engine: Engine, config: AppConfig) {
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

    loop {
        tokio::select! {
            client_msg = read.next() => {
                match client_msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(cmd) = serde_json::from_str::<Command>(&text) {
                            match cmd.action {
                                CommandAction::Subscribe => {
                                    let symbol = cmd.channel.clone();
                                    
                                    if engine.request_ingestion(symbol.clone()).await {
                                        println!("Starting ingestion for new symbol: {}", symbol);
                                        let engine_clone = engine.clone();
                                        let symbol_clone = symbol.clone();
                                        let app_config = config.clone();
                                        
                                        let stream_config = cmd.config.unwrap_or_else(|| app_config.get_stream_config());
                                        
                                        tokio::spawn(async move {
                                            binance_spot::connect_binance(symbol_clone, engine_clone, stream_config, app_config).await;
                                        });
                                    }

                                    subscribed_topics.insert(symbol.clone());

                                    // Send Snapshots
                                    if let Some(book) = engine.get_order_book(&symbol).await {
                                        if let Ok(json) = serde_json::to_string(&MarketData::OrderBook(book)) {
                                            let _ = write.send(Message::Text(json)).await;
                                        }
                                    }

                                    let trades = engine.get_recent_trades(&symbol).await;
                                    for trade in trades {
                                        if let Ok(json) = serde_json::to_string(&MarketData::Trade(trade)) {
                                            let _ = write.send(Message::Text(json)).await;
                                        }
                                    }

                                    let agg_trades = engine.get_recent_agg_trades(&symbol).await;
                                    for trade in agg_trades {
                                        if let Ok(json) = serde_json::to_string(&MarketData::AggTrade(trade)) {
                                            let _ = write.send(Message::Text(json)).await;
                                        }
                                    }

                                    let candles = engine.get_recent_candles(&symbol).await;
                                    for candle in candles {
                                        if let Ok(json) = serde_json::to_string(&MarketData::Candle(candle)) {
                                            let _ = write.send(Message::Text(json)).await;
                                        }
                                    }
                                    
                                    // #2. Use configured history limit
                                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
                                    let initial_history = engine.get_history(&symbol, now, config.server_history_fetch_limit).await;
                                    if !initial_history.is_empty() {
                                        if let Ok(json) = serde_json::to_string(&MarketData::HistoricalCandles(initial_history)) {
                                            let _ = write.send(Message::Text(json)).await;
                                        }
                                    }
                                }
                                CommandAction::Unsubscribe => {
                                    subscribed_topics.remove(&cmd.channel);
                                }
                                CommandAction::FetchHistory => {
                                    let symbol = cmd.channel.clone();
                                    let end_time = cmd.end_time.unwrap_or_else(|| {
                                        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64
                                    });

                                    // #3. Use configured history limit
                                    let history = engine.get_history(&symbol, end_time, config.server_history_fetch_limit).await;
                                    if let Ok(json) = serde_json::to_string(&MarketData::HistoricalCandles(history)) {
                                        let _ = write.send(Message::Text(json)).await;
                                    }
                                }
                            }
                        }
                    }
                    _ => break,
                }
            }

            engine_msg = engine_rx.recv() => {
                match engine_msg {
                    Ok((json_str, data_arc)) => {
                        let symbol: &String = match &*data_arc {
                            MarketData::OrderBook(book) => &book.symbol,
                            MarketData::Trade(trade) => &trade.symbol,
                            MarketData::AggTrade(trade) => &trade.symbol,
                            MarketData::Candle(candle) => &candle.symbol,
                            MarketData::HistoricalCandles(_) => continue, 
                        };

                        if subscribed_topics.contains(symbol) {
                            if write.send(Message::Text(json_str)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }
    
    println!("Client disconnected");
}