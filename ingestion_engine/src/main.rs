// @file: main.rs
// @description: Entry point. Orchestrates Engine, Server, and CLI Input.
// @author: v5 helper
// ingestion_engine/src/main.rs

mod models;
mod engine;
mod exchanges;
mod server;
mod interfaces;

#[cfg(test)]
mod tests; 

use crate::engine::Engine;
use crate::models::StreamConfig; 
use tokio::task;
use tokio::io::{AsyncBufReadExt, BufReader};


#[tokio::main]
async fn main() {
    env_logger::init();
    let engine: Engine = Engine::new();

    println!("Starting QuantSystem Ingestion Engine...");
    println!("Interactive Mode: Type a symbol (e.g., SOLUSDT) and press Enter to ingest.");

    // #1. Spawn Default Handlers
    let defaults: Vec<String> = vec!["BTCUSDT".to_string()];

    // Example: BTC gets EVERYTHING (Default Config)
    let btc_config = StreamConfig::default();

    for symbol in defaults {
        if engine.request_ingestion(symbol.clone()).await {
            let engine_clone = engine.clone();
            let symbol_clone = symbol.clone();
            let config_clone = btc_config.clone();
            
            println!("Spawning default ingestion for: {}", symbol);
            
            task::spawn(async move {
                exchanges::binance::connect_binance(symbol_clone, engine_clone, config_clone).await;
            });
        }
    }

    // #2. Spawn WebSocket Server
    let engine_clone_server = engine.clone();
    let server_task = task::spawn(async move {
        server::start_server(engine_clone_server).await;
    });

    // #3. CLI Input Task
    let engine_clone_cli = engine.clone();
    let cli_task = task::spawn(async move {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            if reader.read_line(&mut line).await.is_ok() {
                let input: String = line.trim().to_uppercase();
                
                if !input.is_empty() {
                    if engine_clone_cli.request_ingestion(input.clone()).await {
                        println!(">> Manual command received. Spawning handler for: {}", input);
                        
                        let engine_connector = engine_clone_cli.clone();
                        let symbol_connector = input.clone();
                        
                        // Example: Custom Config for Manual Ingestion (e.g., Disable AggTrades)
                        let custom_config = StreamConfig::default();
                        // To customize: make variable mutable and uncomment below
                        // custom_config.agg_trades = false; 
                        
                        task::spawn(async move {
                            exchanges::binance::connect_binance(symbol_connector, engine_connector, custom_config).await;
                        });
                    } else {
                        println!(">> Symbol {} is already active.", input);
                    }
                }
            }
        }
    });

    let _ = tokio::join!(server_task, cli_task);
}