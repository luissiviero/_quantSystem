// @file: main.rs
// @description: Entry point. Orchestrates Engine, Server, and CLI Input.
// @author: v5 helper
// ingestion_engine/src/main.rs

mod models;
mod engine;
mod exchanges;
mod server;
mod interfaces;

// #1. Test Module Configuration
#[cfg(test)]
mod tests; 

use crate::engine::Engine;
use tokio::task;
use tokio::io::{AsyncBufReadExt, BufReader};

//
// MAIN EXECUTION
//

#[tokio::main]
async fn main() {
    // #2. Initialize Shared Engine State
    let engine: Engine = Engine::new();

    println!("Starting QuantSystem Ingestion Engine...");
    println!("Interactive Mode: Type a symbol (e.g., SOLUSDT) and press Enter to ingest.");

    // #3. Spawn Default Handlers (BTCUSDT)
    let defaults: Vec<String> = vec!["BTCUSDT".to_string()];

    for symbol in defaults {
        if engine.request_ingestion(symbol.clone()).await {
            let engine_clone: Engine = engine.clone();
            let symbol_clone: String = symbol.clone();
            
            println!("Spawning default ingestion for: {}", symbol);
            
            task::spawn(async move {
                exchanges::binance::connect_binance(symbol_clone, engine_clone).await;
            });
        }
    }

    // #4. Spawn WebSocket Server
    let engine_clone_server: Engine = engine.clone();
    let server_task = task::spawn(async move {
        server::start_server(engine_clone_server).await;
    });

    // #5. CLI Input Task (Manual Control)
    let engine_clone_cli: Engine = engine.clone();
    let cli_task = task::spawn(async move {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            // This awaits until you press Enter
            if reader.read_line(&mut line).await.is_ok() {
                let input: String = line.trim().to_uppercase();
                
                if !input.is_empty() {
                    // Check if already running
                    if engine_clone_cli.request_ingestion(input.clone()).await {
                        println!(">> Manual command received. Spawning handler for: {}", input);
                        
                        let engine_connector: Engine = engine_clone_cli.clone();
                        let symbol_connector: String = input.clone();
                        
                        // Spawn the new connection
                        task::spawn(async move {
                            exchanges::binance::connect_binance(symbol_connector, engine_connector).await;
                        });
                    } else {
                        println!(">> Symbol {} is already active.", input);
                    }
                }
            }
        }
    });

    // #6. Keep Alive
    let _ = tokio::join!(server_task, cli_task);
}