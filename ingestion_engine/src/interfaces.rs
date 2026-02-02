// @file: interfaces.rs
// @description: Defines traits for data processing (if applicable).
// @author: v5 helper

use crate::models::MarketData;
use async_trait::async_trait;

//
// TRAIT DEFINITIONS
//

#[allow(dead_code)]
#[async_trait]
pub trait DataProcessor {
    // 1. Process incoming market data
    async fn process(&self, data: MarketData);
    
    // 2. Handle errors
    fn on_error(&self, error: String);
}

//
// MOCK IMPLEMENTATION (Example)
//

#[allow(dead_code)]
pub struct LoggerProcessor;

#[async_trait]
impl DataProcessor for LoggerProcessor {
    async fn process(&self, data: MarketData) {
        println!("Processing data: {:?}", data);
    }

    fn on_error(&self, error: String) {
        eprintln!("Processor error: {}", error);
    }
}