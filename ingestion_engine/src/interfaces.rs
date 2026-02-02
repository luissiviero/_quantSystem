// ingestion_engine/src/interfaces.rs
use async_trait::async_trait;
use tokio::sync::mpsc;
use crate::models::{DataType, MarketData};

#[async_trait]
pub trait MarketSource: Send + Sync {
    // Start the internal loop. Pass the Data Pipe (sender) so the source can push data.
    async fn start(&mut self, data_pipe: mpsc::Sender<MarketData>) -> Result<(), String>;

    // The Control Knob. The Engine calls this to tell the source what to watch.
    async fn subscribe(&self, symbol: &str, data_type: DataType) -> Result<(), String>;
}