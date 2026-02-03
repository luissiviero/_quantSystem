// @file: engine.rs
// @description: Core engine managing market state with pre-serialized broadcasting to reduce egress CPU load.
// @author: v5 helper
// ingestion_engine\src\engine.rs

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use crate::models::{OrderBook, Trade, MarketData};
use crate::interfaces::DataProcessor;


//
// TYPE DEFINITIONS
//

pub type SharedOrderBook = Arc<RwLock<HashMap<String, OrderBook>>>;
pub type SharedTrades = Arc<RwLock<HashMap<String, VecDeque<Trade>>>>;
pub type ProcessorList = Arc<RwLock<Vec<Box<dyn DataProcessor>>>>;


//
// ENGINE STRUCT
//

#[derive(Clone)]
pub struct Engine {
    pub order_books: SharedOrderBook,
    pub recent_trades: SharedTrades,
    pub processors: ProcessorList,
    // #1. Optimization: Broadcast serialized JSON (String) to avoid redundant serialization
    pub tx: broadcast::Sender<(String, Arc<MarketData>)>, 
}


impl Engine {
    //
    // INITIALIZATION
    //

    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(1000);

        Engine {
            order_books: Arc::new(RwLock::new(HashMap::new())),
            recent_trades: Arc::new(RwLock::new(HashMap::new())),
            processors: Arc::new(RwLock::new(Vec::new())),
            tx,
        }
    }


    //
    // CONFIGURATION & PLUGINS
    //

    pub async fn register_processor(&self, processor: Box<dyn DataProcessor>) {
        // #1. Acquire write lock on processors list
        let mut processors_guard = self.processors.write().await;

        // #2. Insert new processor
        processors_guard.push(processor);
    }


    //
    // STATE MANAGEMENT LOGIC
    //

    pub async fn update_order_book(&self, symbol: String, book: OrderBook) {
        // #1. Update internal state
        {
            let mut books_guard = self.order_books.write().await;
            books_guard.insert(symbol.clone(), book.clone());
        }
        
        // #2. Pre-serialize for egress efficiency
        let market_data: MarketData = MarketData::OrderBook(book);
        if let Ok(json) = serde_json::to_string(&market_data) {
            let msg: Arc<MarketData> = Arc::new(market_data);
            
            // #3. Notify & Broadcast
            self.notify_processors(msg.clone()).await;
            let _ = self.tx.send((json, msg));
        }
    }


    pub async fn add_trade(&self, symbol: String, trade: Trade) {
        // #1. Update internal state
        {
            let mut trades_guard = self.recent_trades.write().await;
            let trades_queue = trades_guard.entry(symbol).or_insert_with(VecDeque::new);
            trades_queue.push_back(trade.clone());
            if trades_queue.len() > 100 {
                trades_queue.pop_front();
            }
        }

        // #2. Pre-serialize
        let market_data: MarketData = MarketData::Trade(trade);
        if let Ok(json) = serde_json::to_string(&market_data) {
            let msg: Arc<MarketData> = Arc::new(market_data);
            
            // #3. Notify & Broadcast
            self.notify_processors(msg.clone()).await;
            let _ = self.tx.send((json, msg));
        }
    }


    async fn notify_processors(&self, data: Arc<MarketData>) {
        let processors = self.processors.read().await;
        for processor in processors.iter() {
            processor.process(data.clone()).await;
        }
    }


    //
    // DATA ACCESSORS
    //

    pub async fn get_order_book(&self, symbol: &str) -> Option<OrderBook> {
        let books = self.order_books.read().await;
        books.get(symbol).cloned()
    }


    pub async fn get_recent_trades(&self, symbol: &str) -> Vec<Trade> {
        let trades = self.recent_trades.read().await;
        match trades.get(symbol) {
            Some(queue) => queue.iter().cloned().collect(),
            None => Vec::new(),
        }
    }
}