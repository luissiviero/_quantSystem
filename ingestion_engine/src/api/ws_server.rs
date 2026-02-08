// @file: ingestion_engine/src/api/ws_server.rs
// @description: WebSocket server updating to use ConnectorManager.
// @author: LAS.

use std::net::SocketAddr;
use std::collections::HashSet;
use tokio::net::{TcpListener, TcpStream};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use crate::core::engine::Engine;
use crate::core::models::{Command, CommandAction, MarketData}; 
use crate::connectors; // Import the factory module
use crate::utils::config::AppConfig;


pub async fn start_server(engine: Engine, config: AppConfig) {
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
                            
                            // #1. Construct the Unique ID based on Command params
                            // Format: EXCHANGE_MARKET_SYMBOL (e.g., BINANCE_SPOT_BTCUSDT)
                            let unique_id: String = format!("{}_{}_{}", cmd.exchange, cmd.market_type, cmd.channel).to_uppercase();

                            match cmd.action {
                                CommandAction::Subscribe => {
                                    
                                    // #2. Request Ingestion via Engine
                                    if engine.request_ingestion(unique_id.clone()).await {
                                        println!("Starting ingestion for: {}", unique_id);
                                        
                                        let engine_clone = engine.clone();
                                        let symbol_clone = cmd.channel.clone();
                                        let app_config = config.clone();
                                        let stream_config = cmd.config.unwrap_or_else(|| app_config.get_stream_config());
                                        
                                        // #3. Spawn via Connector Manager
                                        connectors::spawn_connector(
                                            cmd.exchange,
                                            cmd.market_type,
                                            symbol_clone, // Pass raw symbol (BTCUSDT)
                                            engine_clone,
                                            stream_config,
                                            app_config
                                        ).await;
                                    }

                                    subscribed_topics.insert(unique_id.clone());

                                    // #4. Send Snapshots (using unique_id)
                                    if let Some(book) = engine.get_order_book(&unique_id).await {
                                        if let Ok(json) = serde_json::to_string(&MarketData::OrderBook(book)) {
                                            let _ = write.send(Message::Text(json)).await;
                                        }
                                    }
                                    // ... (Repeat for trades, candles using unique_id) ...
                                }
                                CommandAction::Unsubscribe => {
                                    subscribed_topics.remove(&unique_id);
                                }
                                CommandAction::FetchHistory => {
                                    // History fetch logic ...
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
}