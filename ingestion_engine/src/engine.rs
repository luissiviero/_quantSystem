// ingestion_engine/src/engine.rs
use tokio::sync::mpsc;
use crate::models::{Command, MarketData};
use crate::interfaces::MarketSource;
use log::{info, error};

pub struct IngestionEngine {
    sources: Vec<Box<dyn MarketSource>>,
}

impl IngestionEngine {
    pub fn new() -> Self {
        Self { sources: Vec::new() }
    }

    pub fn add_source(&mut self, source: Box<dyn MarketSource>) {
        self.sources.push(source);
    }

    pub async fn run(
        mut self, 
        mut command_rx: mpsc::Receiver<Command>, 
        data_tx: mpsc::Sender<MarketData>
    ) {
        info!("Engine starting...");

        for source in &mut self.sources {
            if let Err(e) = source.start(data_tx.clone()).await {
                error!("Failed to start source: {}", e);
            }
        }

        info!("Engine listening for commands...");
        while let Some(cmd) = command_rx.recv().await {
            info!("Engine received command: {:?}", cmd);
            match cmd {
                Command::Subscribe { symbol, data_type } => {
                    for source in &self.sources {
                        if let Err(e) = source.subscribe(&symbol, data_type.clone()).await {
                            error!("Subscription failed: {}", e);
                        }
                    }
                }
            }
        }
    }
}