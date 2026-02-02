// ingestion_engine/src/server.rs
use tokio::net::TcpListener;
use tokio::sync::mpsc::{UnboundedSender, Receiver, Sender};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::accept_async;
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::collections::HashMap;
use std::sync::Arc;
use crate::models::{Command, MarketData};
use log::{info, error};

type PeerMap = Arc<Mutex<HashMap<SocketAddr, UnboundedSender<String>>>>;

pub struct Server {
    peers: PeerMap,
}

impl Server {
    pub fn new() -> Self {
        Self {
            peers: PeerMap::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn run(
        self, 
        mut data_rx: Receiver<MarketData>, 
        command_tx: Sender<Command>
    ) {
        let addr = "127.0.0.1:3000";
        let listener = TcpListener::bind(&addr).await.expect("Failed to bind");
        info!("WebSocket Server running on: {}", addr);

        let peers = self.peers.clone();

        // TASK 1: Broadcast Data (Engine -> Frontend)
        let broadcast_peers = peers.clone();
        tokio::spawn(async move {
            while let Some(data) = data_rx.recv().await {
                let msg = serde_json::to_string(&data).unwrap_or_default();
                let locked_peers = broadcast_peers.lock().await;
                
                // info!("Broadcasting data to {} clients", locked_peers.len()); // DEBUG: Very spammy, use only if needed
                for peer in locked_peers.values() {
                    let _ = peer.send(msg.clone());
                }
            }
        });

        // TASK 2: Accept New Connections
        while let Ok((stream, addr)) = listener.accept().await {
            let peers = peers.clone();
            let cmd_tx = command_tx.clone();

            tokio::spawn(async move {
                info!("New connection: {}", addr);
                let ws_stream = match accept_async(stream).await {
                    Ok(ws) => ws,
                    Err(e) => {
                        error!("Error during websocket handshake: {}", e);
                        return;
                    }
                };

                let (mut ws_sender, mut ws_receiver) = ws_stream.split();
                let (tx, mut rx) = mpsc::unbounded_channel();
                peers.lock().await.insert(addr, tx);

                let send_task = tokio::spawn(async move {
                    while let Some(msg) = rx.recv().await {
                        if ws_sender.send(tokio_tungstenite::tungstenite::Message::Text(msg)).await.is_err() {
                            break; 
                        }
                    }
                });

                while let Some(msg) = ws_receiver.next().await {
                    match msg {
                        Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                            info!("Received from Frontend: {}", text); // DEBUG LOG
                            if let Ok(cmd) = serde_json::from_str::<Command>(&text) {
                                let _ = cmd_tx.send(cmd).await;
                            } else {
                                error!("Failed to parse frontend command: {}", text);
                            }
                        }
                        Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
                        _ => {}
                    }
                }

                send_task.abort();
                peers.lock().await.remove(&addr);
                info!("Disconnected: {}", addr);
            });
        }
    }
}