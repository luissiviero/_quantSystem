// @file: ingestion_engine/src/main.rs
// @description: Entry point updated to support manual commands with Exchange/Market context.
// @author: LAS.

use ingestion_engine::core::engine::Engine;
use ingestion_engine::api::ws_server;
use ingestion_engine::connectors; // Use Factory
use ingestion_engine::utils::config::AppConfig;
use ingestion_engine::core::models::{Exchange, MarketType}; // Import Enums

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
    println!("Interactive Mode: Type 'EXCHANGE:MARKET:SYMBOL' (e.g., BINANCE:SPOT:SOLUSDT) or just SYMBOL (defaults to Binance Spot).");

    // #2. Spawn Defaults (Assuming Binance Spot for legacy defaults)
    let defaults: Vec<String> = config.default_symbols.clone();
    let default_stream_config = config.get_stream_config();

    for symbol in defaults {
        let unique_id: String = format!("BINANCE_SPOT_{}", symbol).to_uppercase();
        
        if engine.request_ingestion(unique_id).await {
            let engine_clone = engine.clone();
            let symbol_clone = symbol.clone();
            let stream_cfg = default_stream_config.clone();
            let app_cfg = config.clone();
            
            task::spawn(async move {
                connectors::spawn_connector(
                    Exchange::Binance,
                    MarketType::Spot,
                    symbol_clone,
                    engine_clone,
                    stream_cfg,
                    app_cfg
                ).await;
            });
        }
    }

    // #3. Spawn Server
    let engine_server = engine.clone();
    let config_server = config.clone(); 
    let server_task = task::spawn(async move {
        ws_server::start_server(engine_server, config_server).await;
    });

    // #4. CLI Input Task
    let engine_cli = engine.clone();
    let config_cli = config.clone();
    let cli_task = task::spawn(async move {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            if reader.read_line(&mut line).await.is_ok() {
                let input: String = line.trim().to_uppercase();
                
                if !input.is_empty() {
                    // Simple parsing logic: EXCHANGE:MARKET:SYMBOL or just SYMBOL
                    let parts: Vec<&str> = input.split(':').collect();
                    
                    let (exchange, market, symbol) = if parts.len() == 3 {
                        let ex = match parts[0] {
                            "BINANCE" => Exchange::Binance,
                            "BYBIT" => Exchange::Bybit,
                            _ => Exchange::Binance
                        };
                        let mk = match parts[1] {
                            "SPOT" => MarketType::Spot,
                            "FUTURE" | "LINEAR" => MarketType::LinearFuture,
                            _ => MarketType::Spot
                        };
                        (ex, mk, parts[2].to_string())
                    } else {
                        (Exchange::Binance, MarketType::Spot, input.clone())
                    };

                    let unique_id = format!("{}_{}_{}", exchange, market, symbol);

                    if engine_cli.request_ingestion(unique_id.clone()).await {
                        println!(">> Spawning handler for: {} ({:?} {:?})", symbol, exchange, market);
                        
                        let eng = engine_cli.clone();
                        let cfg = config_cli.clone();
                        let stream_cfg = cfg.get_stream_config();
                        
                        connectors::spawn_connector(
                            exchange,
                            market,
                            symbol,
                            eng,
                            stream_cfg,
                            cfg
                        ).await;
                    } else {
                        println!(">> {} is already active.", unique_id);
                    }
                }
            }
        }
    });

    let _ = tokio::join!(server_task, cli_task);
}