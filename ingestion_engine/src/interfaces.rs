// @file: interfaces.rs
// @description: Defines traits for data processing (if applicable).
// @author: v5 helper
// ingestion_engine/src/interfaces.rs

use crate::models::MarketData;
use async_trait::async_trait;
use std::sync::Arc;

//
// TRAIT DEFINITIONS
//

#[async_trait]
pub trait DataProcessor: Send + Sync {
    // #1. Process incoming market data
    // Using Arc<MarketData> to match the engine's zero-copy architecture
    async fn process(&self, data: Arc<MarketData>);
    
    // #2. Handle errors
    // FIX: Suppress warning because Engine doesn't call this yet (Reserved for future use)
    #[allow(dead_code)]
    fn on_error(&self, error: String);
}

//
// MOCK IMPLEMENTATION (Example)
//

pub struct LoggerProcessor;

#[async_trait]
impl DataProcessor for LoggerProcessor {
    async fn process(&self, data: Arc<MarketData>) {
        println!("Processor Log: {:?}", data);
    }

    fn on_error(&self, error: String) {
        eprintln!("Processor Error: {}", error);
    }
}