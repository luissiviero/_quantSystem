// @file: main.rs
// @description: Entry point for the ingestion engine. Orchestrates the Engine, Plugins, and Server.
// @author: v5 helper
// ingestion_engine/src/main.rs

mod models;
mod engine;
mod exchanges;
mod server;
mod interfaces;

use crate::engine::Engine;
use crate::interfaces::LoggerProcessor; // Import specific plugins here
use tokio::task;

//
// MAIN EXECUTION
//

#[tokio::main]
async fn main() {
    // #1. Initialize Shared Engine State
    let engine: Engine = Engine::new();

    println!("Starting QuantSystem Ingestion Engine...");

    // #2. REGISTER PLUGINS (The "Plug & Play" Section)
    // You can add as many processors here as you want.
    // The engine stores them as Box<dyn DataProcessor>.
    engine.register_processor(Box::new(LoggerProcessor)).await;

    // #3. Spawn Binance Handler (BTCUSDT)
    // We clone the engine handle for the Binance task
    let engine_clone_binance: Engine = engine.clone();
    let binance_task = task::spawn(async move {
        // Hardcoded symbol for now, could be dynamic later
        exchanges::binance::connect_binance("BTCUSDT".to_string(), engine_clone_binance).await;
    });

    // #4. Spawn WebSocket Server for Frontend
    // We clone the engine handle for the Server task
    let engine_clone_server: Engine = engine.clone();
    let server_task = task::spawn(async move {
        server::start_server(engine_clone_server).await;
    });

    // #5. Await tasks
    // This keeps the main thread alive while the sub-tasks run infinite loops
    // If either task crashes (unlikely with our error handling), the join will return.
    let _ = tokio::join!(binance_task, server_task);
}