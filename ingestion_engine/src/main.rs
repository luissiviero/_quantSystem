mod models;
mod interfaces;
mod exchanges;
mod engine;
mod server;

use crate::engine::IngestionEngine;
use crate::server::Server;
use crate::exchanges::binance::BinanceSource;
use tokio::sync::mpsc;
use log::info; 

#[tokio::main]
async fn main() {
    // 1. Initialize Logger with a default level of "info"
    // This ensures you see the logs even without setting RUST_LOG environment variable
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!(">>> Ingestion Engine is Starting... <<<");

    // 2. Create Channels (The Pipes)
    let (cmd_tx, cmd_rx) = mpsc::channel(100);
    let (data_tx, data_rx) = mpsc::channel(1000);

    // 3. Setup Engine
    let mut engine = IngestionEngine::new();
    let binance = BinanceSource::new();
    engine.add_source(Box::new(binance));

    // 4. Setup Server
    let server = Server::new();

    // 5. Run Components
    tokio::spawn(async move {
        engine.run(cmd_rx, data_tx).await;
    });

    info!(">>> Engine running. Starting Server on port 3000... <<<");

    server.run(data_rx, cmd_tx).await;
}