// @file: ingestion_engine\src\core\interfaces.rs
// @description: Defines traits for data processing and the FrontendSimulator.
// @author: LAS.

use crate::core::models::MarketData;
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
    #[allow(dead_code)]
    fn on_error(&self, error: String);
}