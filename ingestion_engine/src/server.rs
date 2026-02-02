// @file: server.rs
// @description: WebSocket server with broadcast capabilities.
// @author: v5 helper

use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use crate::engine::Engine;
use crate::models::{Command, MarketData};

//
// CONSTANTS
//

const SERVER_ADDR: &str = "127.0.0.1:8080";

//
// SERVER ENTRY POINT
//

pub async fn start_server(engine: Engine) {
    let addr: SocketAddr = SERVER_ADDR.parse().expect("Invalid address");
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");
    
    println!("WebSocket server listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let engine_clone = engine.clone();
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
            eprintln!("Handshake error: {}", e);
            return;
        }
    };

    println!("Client connected");

    let (mut write, mut read) = ws_stream.split();
    
    // 1. Subscribe to Engine Broadcasts
    let mut rx = engine.tx.subscribe();

    // 2. Main Event Loop (Select between incoming Commands and outgoing Broadcasts)
    loop {
        tokio::select! {
            // A. Handle Incoming WebSocket Messages (from Client)
            msg_option = read.next() => {
                match msg_option {
                    Some(Ok(Message::Text(text))) => {
                         // Optional: Handle commands like 'subscribe' here if you want to filter streams
                         // For now, we just log it.
                         if let Ok(cmd) = serde_json::from_str::<Command>(&text) {
                            println!("Client command: {}", cmd.action);
                             // Send initial snapshot on subscribe
                             if cmd.action == "subscribe" {
                                if let Some(book) = engine.get_order_book(&cmd.channel).await {
                                     let response = MarketData::OrderBook(book);
                                     let json = serde_json::to_string(&response).unwrap();
                                     let _ = write.send(Message::Text(json)).await;
                                }
                             }
                         }
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Err(_)) => break,
                    None => break,
                    _ => {}
                }
            }

            // B. Handle Outgoing Engine Events (to Client)
            event_result = rx.recv() => {
                if let Ok(event) = event_result {
                    // Serialize and send
                    if let Ok(json) = serde_json::to_string(&event) {
                        if write.send(Message::Text(json)).await.is_err() {
                            break; // Client disconnected
                        }
                    }
                }
            }
        }
    }
    
    println!("Client disconnected");
}