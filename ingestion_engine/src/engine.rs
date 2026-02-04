// @file: engine.rs
// @description: Core engine managing market state with pre-serialized broadcasting to reduce egress CPU load.
// @author: v5 helper
// ingestion_engine/src/engine.rs

use std::collections::{HashMap, VecDeque, HashSet}; // Added HashSet
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use crate::models::{OrderBook, Trade, AggTrade, Candle, MarketData};
use crate::interfaces::DataProcessor;


//
// TYPE DEFINITIONS
//

pub type SharedOrderBook = Arc<RwLock<HashMap<String, OrderBook>>>;
pub type SharedTrades = Arc<RwLock<HashMap<String, VecDeque<Trade>>>>;
pub type SharedAggTrades = Arc<RwLock<HashMap<String, VecDeque<AggTrade>>>>;
pub type SharedCandles = Arc<RwLock<HashMap<String, HashMap<String, VecDeque<Candle>>>>>; 
pub type ProcessorList = Arc<RwLock<Vec<Box<dyn DataProcessor>>>>;
pub type ActiveIngestions = Arc<RwLock<HashSet<String>>>; // #1. New Type


//
// ENGINE STRUCT
//

#[derive(Clone)]
pub struct Engine {
    pub order_books: SharedOrderBook,
    pub recent_trades: SharedTrades,
    pub recent_agg_trades: SharedAggTrades,
    pub recent_candles: SharedCandles,
    pub processors: ProcessorList,
    pub active_ingestions: ActiveIngestions, // #2. New Field
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
            recent_agg_trades: Arc::new(RwLock::new(HashMap::new())),
            recent_candles: Arc::new(RwLock::new(HashMap::new())),
            processors: Arc::new(RwLock::new(Vec::new())),
            active_ingestions: Arc::new(RwLock::new(HashSet::new())), // #3. Initialize
            tx,
        }
    }


    //
    // CONFIGURATION & PLUGINS
    //
    
    #[allow(dead_code)]
    pub async fn register_processor(&self, processor: Box<dyn DataProcessor>) {
        let mut processors_guard = self.processors.write().await;
        processors_guard.push(processor);
    }

    // #4. Ingestion Control Logic (Thread-Safe Check-and-Set)
    // Returns true if the symbol was NOT present and has now been added (caller should spawn task).
    // Returns false if the symbol was already present (caller should do nothing).
    pub async fn request_ingestion(&self, symbol: String) -> bool {
        let mut active_guard = self.active_ingestions.write().await;
        active_guard.insert(symbol)
    }


    //
    // STATE MANAGEMENT LOGIC
    //

    pub async fn update_order_book(&self, symbol: String, book: OrderBook) {
        {
            let mut books_guard = self.order_books.write().await;
            books_guard.insert(symbol.clone(), book.clone());
        }
        
        let market_data: MarketData = MarketData::OrderBook(book);
        if let Ok(json) = serde_json::to_string(&market_data) {
            let msg: Arc<MarketData> = Arc::new(market_data);
            self.notify_processors(msg.clone()).await;
            let _ = self.tx.send((json, msg));
        }
    }


    pub async fn add_trade(&self, symbol: String, trade: Trade) {
        {
            let mut trades_guard = self.recent_trades.write().await;
            let trades_queue: &mut VecDeque<Trade> = trades_guard
                .entry(symbol)
                .or_insert_with(VecDeque::new);
            
            trades_queue.push_back(trade.clone());
            if trades_queue.len() > 100 {
                trades_queue.pop_front();
            }
        }

        let market_data: MarketData = MarketData::Trade(trade);
        if let Ok(json) = serde_json::to_string(&market_data) {
            let msg: Arc<MarketData> = Arc::new(market_data);
            self.notify_processors(msg.clone()).await;
            let _ = self.tx.send((json, msg));
        }
    }


    pub async fn add_agg_trade(&self, symbol: String, trade: AggTrade) {
        {
            let mut trades_guard = self.recent_agg_trades.write().await;
            let trades_queue: &mut VecDeque<AggTrade> = trades_guard
                .entry(symbol)
                .or_insert_with(VecDeque::new);
            
            trades_queue.push_back(trade.clone());
            if trades_queue.len() > 100 {
                trades_queue.pop_front();
            }
        }

        let market_data: MarketData = MarketData::AggTrade(trade);
        if let Ok(json) = serde_json::to_string(&market_data) {
            let msg: Arc<MarketData> = Arc::new(market_data);
            self.notify_processors(msg.clone()).await;
            let _ = self.tx.send((json, msg));
        }
    }


    pub async fn add_candle(&self, symbol: String, candle: Candle) {
        {
            let mut candles_guard = self.recent_candles.write().await;
            let interval_map: &mut HashMap<String, VecDeque<Candle>> = candles_guard
                .entry(symbol.clone())
                .or_insert_with(HashMap::new);

            let candles_queue: &mut VecDeque<Candle> = interval_map
                .entry(candle.interval.clone())
                .or_insert_with(VecDeque::new);

            candles_queue.push_back(candle.clone());

            if candles_queue.len() > 5000 {
                candles_queue.pop_front();
            }
        }

        let market_data: MarketData = MarketData::Candle(candle);
        if let Ok(json) = serde_json::to_string(&market_data) {
            let msg: Arc<MarketData> = Arc::new(market_data);
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


    pub async fn get_recent_agg_trades(&self, symbol: &str) -> Vec<AggTrade> {
        let trades = self.recent_agg_trades.read().await;
        match trades.get(symbol) {
            Some(queue) => queue.iter().cloned().collect(),
            None => Vec::new(),
        }
    }


    pub async fn get_recent_candles(&self, symbol: &str) -> Vec<Candle> {
        let candles_guard = self.recent_candles.read().await;
        let mut result: Vec<Candle> = Vec::new();

        if let Some(interval_map) = candles_guard.get(symbol) {
            for queue in interval_map.values() {
                result.extend(queue.iter().cloned());
            }
        }
        result
    }

    // #5. NEW: Historical Accessor for Infinite Scroll
    pub async fn get_history(&self, symbol: &str, end_time: u64, limit: usize) -> Vec<Candle> {
        let candles_guard = self.recent_candles.read().await;
        let mut result: Vec<Candle> = Vec::new();

        if let Some(interval_map) = candles_guard.get(symbol) {
            // Flatten all intervals for the symbol
            for queue in interval_map.values() {
                // Filter: Only take candles strictly OLDER than end_time
                let older_candles = queue.iter().filter(|c| c.start_time < end_time);
                result.extend(older_candles.cloned());
            }
        }

        // Sort by time to ensure order after flattening
        result.sort_by_key(|c| c.start_time);

        // Take the *last* 'limit' items (closest to the end_time)
        if result.len() > limit {
            let start = result.len() - limit;
            result.drain(start..).collect()
        } else {
            result
        }
    }
}