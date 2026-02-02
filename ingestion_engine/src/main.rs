// @file: main.rs
// @description: Entry point for the ingestion engine.
// @author: v5 helper

mod models;
mod engine;
mod exchanges;
mod server;
mod interfaces;

use crate::engine::Engine;
use tokio::task;

//
// MAIN EXECUTION
//

#[tokio::main]
async fn main() {
    // 1. Initialize Shared Engine State
    let engine: Engine = Engine::new();

    println!("Starting QuantSystem Ingestion Engine...");

    // 2. Spawn Binance Handler (BTCUSDT)
    let engine_clone_binance: Engine = engine.clone();
    let binance_task = task::spawn(async move {
        exchanges::binance::connect_binance("BTCUSDT".to_string(), engine_clone_binance).await;
    });

    // 3. Spawn WebSocket Server for Frontend
    let engine_clone_server: Engine = engine.clone();
    let server_task = task::spawn(async move {
        server::start_server(engine_clone_server).await;
    });

    // 4. Await tasks
    let _ = tokio::join!(binance_task, server_task);
}