// @file: ingestion_engine/src/api/ws_server.rs
// @description: WebSocket server with full snapshots including FundingRate and OpenInterest.
// @author: LAS.

use std::net::SocketAddr;
use std::collections::HashSet;
use tokio::net::{TcpListener, TcpStream};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use crate::core::engine::Engine;
use crate::core::models::{Command, CommandAction, MarketData}; 
use crate::connectors::{self, binance_rest}; 
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
                            
                            let unique_id: String = format!("{}_{}_{}", cmd.exchange, cmd.market_type, cmd.channel).to_uppercase();

                            match cmd.action {
                                CommandAction::Subscribe => {
                                    if engine.request_ingestion(unique_id.clone()).await {
                                        println!("Starting ingestion for: {}", unique_id);
                                        let engine_clone = engine.clone();
                                        let symbol_clone = cmd.channel.clone();
                                        let app_config = config.clone();
                                        let stream_config = cmd.config.unwrap_or_else(|| app_config.get_stream_config());
                                        
                                        connectors::spawn_connector(
                                            cmd.exchange,
                                            cmd.market_type,
                                            symbol_clone, 
                                            engine_clone,
                                            stream_config,
                                            app_config
                                        ).await;
                                    }

                                    subscribed_topics.insert(unique_id.clone());

                                    //
                                    // #1. EXISTING SNAPSHOTS
                                    //

                                    // Order Book
                                    if let Some(book) = engine.get_order_book(&unique_id).await {
                                        if let Ok(json) = serde_json::to_string(&MarketData::OrderBook(book)) {
                                            let _ = write.send(Message::Text(json)).await;
                                        }
                                    }

                                    // Recent Trades
                                    let recent_trades = engine.get_recent_trades(&unique_id).await;
                                    for trade in recent_trades {
                                        if let Ok(json) = serde_json::to_string(&MarketData::Trade(trade)) {
                                            let _ = write.send(Message::Text(json)).await;
                                        }
                                    }

                                    // Recent AggTrades
                                    let recent_agg = engine.get_recent_agg_trades(&unique_id).await;
                                    for trade in recent_agg {
                                        if let Ok(json) = serde_json::to_string(&MarketData::AggTrade(trade)) {
                                            let _ = write.send(Message::Text(json)).await;
                                        }
                                    }

                                    // Recent Candles
                                    let recent_candles = engine.get_recent_candles(&unique_id).await;
                                    for candle in recent_candles {
                                        if let Ok(json) = serde_json::to_string(&MarketData::Candle(candle)) {
                                            let _ = write.send(Message::Text(json)).await;
                                        }
                                    }
                                    
                                    //
                                    // #2. NEW FEATURE SNAPSHOTS
                                    //
                                    
                                    // Ticker
                                    if let Some(ticker) = engine.get_ticker(&unique_id).await {
                                        if let Ok(json) = serde_json::to_string(&MarketData::Ticker(ticker)) {
                                            let _ = write.send(Message::Text(json)).await;
                                        }
                                    }
                                    
                                    // Book Ticker
                                    if let Some(bt) = engine.get_book_ticker(&unique_id).await {
                                        if let Ok(json) = serde_json::to_string(&MarketData::BookTicker(bt)) {
                                            let _ = write.send(Message::Text(json)).await;
                                        }
                                    }

                                    // Mark Price
                                    if let Some(mp) = engine.get_mark_price(&unique_id).await {
                                        if let Ok(json) = serde_json::to_string(&MarketData::MarkPrice(mp)) {
                                            let _ = write.send(Message::Text(json)).await;
                                        }
                                    }

                                    // Funding Rate
                                    if let Some(fr) = engine.get_funding_rate(&unique_id).await {
                                        if let Ok(json) = serde_json::to_string(&MarketData::FundingRate(fr)) {
                                            let _ = write.send(Message::Text(json)).await;
                                        }
                                    }

                                    // Open Interest
                                    if let Some(oi) = engine.get_open_interest(&unique_id).await {
                                        if let Ok(json) = serde_json::to_string(&MarketData::OpenInterest(oi)) {
                                            let _ = write.send(Message::Text(json)).await;
                                        }
                                    }

                                    // Recent Liquidations
                                    let recent_liqs = engine.get_recent_liquidations(&unique_id).await;
                                    for liq in recent_liqs {
                                        if let Ok(json) = serde_json::to_string(&MarketData::Liquidation(liq)) {
                                            let _ = write.send(Message::Text(json)).await;
                                        }
                                    }
                                }
                                CommandAction::Unsubscribe => {
                                    subscribed_topics.remove(&unique_id);
                                }
                                CommandAction::FetchHistory => {
                                    println!("Fetching history for {}", unique_id);
                                    
                                    // #1. Determine Params
                                    // Use config interval or default to 1m
                                    let interval = match &cmd.config {
                                        Some(cfg) => cfg.kline_intervals.first().cloned().unwrap_or_else(|| "1m".to_string()),
                                        None => "1m".to_string(),
                                    };
                                    
                                    // #2. Call REST API
                                    let fetch_result = binance_rest::fetch_binance_history(
                                        &cmd.channel, 
                                        cmd.market_type, 
                                        &interval, 
                                        config.server_history_fetch_limit
                                    ).await;

                                    match fetch_result {
                                        Ok(candles) => {
                                            println!("Fetched {} candles for {}", candles.len(), unique_id);
                                            
                                            // #3. Load into Engine (No Broadcast)
                                            engine.load_historical_candles(unique_id.clone(), candles.clone()).await;

                                            // #4. Send to Requesting Client ONLY
                                            // Wrap in MarketData::HistoricalCandles
                                            let response = MarketData::HistoricalCandles(candles);
                                            if let Ok(json) = serde_json::to_string(&response) {
                                                let _ = write.send(Message::Text(json)).await;
                                            }
                                        },
                                        Err(e) => {
                                            eprintln!("History fetch failed: {}", e);
                                        }
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
                            MarketData::HistoricalCandles(_) => continue, // Do not broadcast history
                            
                            // NEW VARIANTS
                            MarketData::Ticker(t) => &t.symbol,
                            MarketData::BookTicker(t) => &t.symbol,
                            MarketData::MarkPrice(t) => &t.symbol,
                            MarketData::Liquidation(t) => &t.symbol,
                            MarketData::FundingRate(t) => &t.symbol,
                            MarketData::OpenInterest(t) => &t.symbol,
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