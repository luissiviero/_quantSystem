// @file: src/main.rs
// @description: Entry point. Sets up modules and shared state.

mod models;
mod ingestion;
mod server;

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use crate::models::GlobalSnapshot; // Import the new struct

#[tokio::main]
async fn main() {
    // #
    // # 1. SHARED STATE (Composite)
    // #
    // Instead of Option<String>, we hold the GlobalSnapshot struct.
    let latest_data: Arc<RwLock<GlobalSnapshot>> = Arc::new(RwLock::new(GlobalSnapshot::default()));
    
    // Broadcast channel still sends JSON Strings (serialized snapshot)
    let (tx, _rx) = broadcast::channel::<String>(100);

    // #
    // # 2. START INGESTION (Binance)
    // #
    let data_writer = latest_data.clone();
    tokio::spawn(async move {
        ingestion::start_ingestion(data_writer).await;
    });

    // #
    // # 3. START BROADCASTER (Throttler)
    // #
    let data_reader = latest_data.clone();
    let tx_publisher = tx.clone();
    tokio::spawn(async move {
        server::broadcast_snapshot_loop(data_reader, tx_publisher).await;
    });

    // #
    // # 4. START SERVER (Localhost)
    // #
    server::start_server(tx).await;
}