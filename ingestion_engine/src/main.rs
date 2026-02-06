// @file: ingestion_engine/src/main.rs
// @description: Entry point. Orchestrates Engine, Server, and CLI Input.
// @author: LAS.


use ingestion_engine::core::engine::Engine;
use ingestion_engine::core::models::StreamConfig; 
use ingestion_engine::api::ws_server;
use ingestion_engine::connectors::binance;
use ingestion_engine::utils::config::AppConfig;

use tokio::task;
use tokio::io::{AsyncBufReadExt, BufReader};
use std::env;

//
// MAIN ENTRY POINT
//

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    // #1. Load Config
    let config: AppConfig = AppConfig::load().expect("Failed to load configuration");

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", &config.log_level);
    }
    env_logger::init();

    let engine: Engine = Engine::new(&config);

    println!("Starting QuantSystem Ingestion Engine...");
    println!("Log Level: {}", config.log_level);
    println!("Interactive Mode: Type a symbol (e.g., SOLUSDT) and press Enter to ingest.");

    let defaults: Vec<String> = config.default_symbols.clone();
    
    // CHANGED: Use helper method instead of StreamConfig::default()
    let default_stream_config: StreamConfig = config.get_stream_config();

    for symbol in defaults {
        if engine.request_ingestion(symbol.clone()).await {
            let engine_clone = engine.clone();
            let symbol_clone = symbol.clone();
            let stream_config_clone = default_stream_config.clone();
            let app_config_clone = config.clone(); 
            
            println!("Spawning default ingestion for: {}", symbol);
            
            task::spawn(async move {
                binance::connect_binance(symbol_clone, engine_clone, stream_config_clone, app_config_clone).await;
            });
        }
    }

    // #2. Spawn Server
    let engine_clone_server = engine.clone();
    let config_clone_server = config.clone(); 
    let server_task = task::spawn(async move {
        ws_server::start_server(engine_clone_server, config_clone_server).await;
    });

    // #3. CLI Input Task
    let engine_clone_cli = engine.clone();
    let config_clone_cli = config.clone();
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
                        let app_config_connector = config_clone_cli.clone();

                        // CHANGED: Use helper method
                        let custom_stream_config = app_config_connector.get_stream_config();
                        
                        task::spawn(async move {
                            binance::connect_binance(symbol_connector, engine_connector, custom_stream_config, app_config_connector).await;
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