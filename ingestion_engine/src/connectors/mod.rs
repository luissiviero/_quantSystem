// @file: ingestion_engine/src/connectors/mod.rs
// @description: Connectors module entry point. Exposes factory and specific exchange implementations.
// @author: LAS.

pub mod factory;
pub mod binance;

// Re-export common traits if needed by other modules (optional)
// pub use factory::ExchangeFactory;