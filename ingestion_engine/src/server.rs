// @file: src/server.rs
// @description: Manages the local TCP/WebSocket server and broadcasts snapshots.

use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};
use futures_util::SinkExt;
use crate::models::GlobalSnapshot;

// #
// # BROADCAST LOOP (The Throttler)
// #
pub async fn broadcast_snapshot_loop(
    shared_state: Arc<RwLock<GlobalSnapshot>>,
    tx: broadcast::Sender<String>
) {
    // Tick every 200ms (5 times a second)
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(200));

    loop {
        interval.tick().await;

        // Serialize the entire state to JSON
        let snapshot_json = {
            let lock = shared_state.read().await;
            serde_json::to_string(&*lock).unwrap_or_default()
        };

        // Broadcast to Frontend
        let _ = tx.send(snapshot_json);
    }
}

// #
// # SERVER LISTENER
// #
pub async fn start_server(tx: broadcast::Sender<String>) {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");
    
    println!("Local Server listening on: {}", addr);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let rx = tx.subscribe();
                tokio::spawn(handle_client(stream, rx));
            }
            Err(e) => eprintln!("Client Connection Error: {}", e),
        }
    }
}

async fn handle_client(stream: TcpStream, mut rx: broadcast::Receiver<String>) {
    let mut ws_stream = match accept_async(stream).await {
        Ok(s) => s,
        Err(_) => return,
    };

    loop {
        match rx.recv().await {
            Ok(msg) => {
                let message = Message::Text(msg.into());
                if ws_stream.send(message).await.is_err() {
                    break; 
                }
            }
            Err(_) => break,
        }
    }
}