// @file: engine.rs
// @description: Optimized core engine with granular locking and complete data accessors.
// @author: v5 helper
// ingestion_engine/src/engine.rs

use std::collections::{HashMap, VecDeque, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use crate::models::{OrderBook, Trade, AggTrade, Candle, MarketData};
use crate::interfaces::DataProcessor;


//
// TYPE DEFINITIONS
//

pub type ProcessorList = Arc<RwLock<Vec<Box<dyn DataProcessor>>>>;
pub type ActiveIngestions = Arc<RwLock<HashSet<String>>>;


//
// GRANULAR SYMBOL STATE
//

pub struct SymbolState {
    pub order_book: RwLock<Option<OrderBook>>,
    pub trades: RwLock<VecDeque<Trade>>,
    pub agg_trades: RwLock<VecDeque<AggTrade>>,
    pub candles: RwLock<HashMap<String, VecDeque<Candle>>>,
}


impl SymbolState {
    fn new() -> Self {
        Self {
            order_book: RwLock::new(None),
            trades: RwLock::new(VecDeque::with_capacity(100)),
            agg_trades: RwLock::new(VecDeque::with_capacity(100)),
            candles: RwLock::new(HashMap::new()),
        }
    }
}


//
// ENGINE STRUCT
//

#[derive(Clone)]
pub struct Engine {
    pub registry: Arc<RwLock<HashMap<String, Arc<SymbolState>>>>,
    pub processors: ProcessorList,
    pub active_ingestions: ActiveIngestions,
    pub tx: broadcast::Sender<(String, Arc<MarketData>)>, 
}


impl Engine {
    //
    // INITIALIZATION
    //

    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(5000); 

        Engine {
            registry: Arc::new(RwLock::new(HashMap::new())),
            processors: Arc::new(RwLock::new(Vec::new())),
            active_ingestions: Arc::new(RwLock::new(HashSet::new())),
            tx,
        }
    }

    pub async fn register_processor(&self, processor: Box<dyn DataProcessor>) {
        let mut processors_guard = self.processors.write().await;
        processors_guard.push(processor);
    }


    //
    // INTERNAL HELPER
    //

    async fn get_or_create_symbol(&self, symbol: &str) -> Arc<SymbolState> {
        // #1. Try to get existing state with a read lock
        {
            let reg = self.registry.read().await;
            if let Some(state) = reg.get(symbol) {
                return state.clone();
            }
        }

        // #2. If not found, upgrade to write lock to insert
        let mut reg = self.registry.write().await;
        reg.entry(symbol.to_string())
            .or_insert_with(|| Arc::new(SymbolState::new()))
            .clone()
    }


    //
    // STATE MANAGEMENT LOGIC
    //

    pub async fn update_order_book(&self, symbol: String, book: OrderBook) {
        let state = self.get_or_create_symbol(&symbol).await;
        
        {
            let mut book_guard = state.order_book.write().await;
            *book_guard = Some(book.clone());
        }
        
        let market_data = MarketData::OrderBook(book);
        self.broadcast_data(market_data).await;
    }


    pub async fn add_trade(&self, symbol: String, trade: Trade) {
        let state = self.get_or_create_symbol(&symbol).await;

        {
            let mut trades_guard = state.trades.write().await;
            if trades_guard.len() >= 100 {
                trades_guard.pop_front();
            }
            trades_guard.push_back(trade.clone());
        }

        let market_data = MarketData::Trade(trade);
        self.broadcast_data(market_data).await;
    }


    pub async fn add_agg_trade(&self, symbol: String, trade: AggTrade) {
        let state = self.get_or_create_symbol(&symbol).await;

        {
            let mut trades_guard = state.agg_trades.write().await;
            if trades_guard.len() >= 100 {
                trades_guard.pop_front();
            }
            trades_guard.push_back(trade.clone());
        }

        let market_data = MarketData::AggTrade(trade);
        self.broadcast_data(market_data).await;
    }


    pub async fn add_candle(&self, symbol: String, candle: Candle) {
        let state = self.get_or_create_symbol(&symbol).await;

        {
            let mut candles_map = state.candles.write().await;
            let queue = candles_map.entry(candle.interval.clone())
                .or_insert_with(|| VecDeque::with_capacity(5000));
            
            if queue.len() >= 5000 {
                queue.pop_front();
            }
            queue.push_back(candle.clone());
        }

        let market_data = MarketData::Candle(candle);
        self.broadcast_data(market_data).await;
    }


    async fn broadcast_data(&self, data: MarketData) {
        if let Ok(json) = serde_json::to_string(&data) {
            let msg = Arc::new(data);
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
        let state = {
            let reg = self.registry.read().await;
            reg.get(symbol).cloned()
        };

        if let Some(s) = state {
            return s.order_book.read().await.clone();
        }
        None
    }


    pub async fn get_recent_trades(&self, symbol: &str) -> Vec<Trade> {
        let state = {
            let reg = self.registry.read().await;
            reg.get(symbol).cloned()
        };

        if let Some(s) = state {
            let guard = s.trades.read().await;
            return guard.iter().cloned().collect();
        }
        Vec::new()
    }


    pub async fn get_recent_agg_trades(&self, symbol: &str) -> Vec<AggTrade> {
        let state = {
            let reg = self.registry.read().await;
            reg.get(symbol).cloned()
        };

        if let Some(s) = state {
            let guard = s.agg_trades.read().await;
            return guard.iter().cloned().collect();
        }
        Vec::new()
    }


    pub async fn get_recent_candles(&self, symbol: &str) -> Vec<Candle> {
        let state = {
            let reg = self.registry.read().await;
            reg.get(symbol).cloned()
        };

        if let Some(s) = state {
            let candles_guard = s.candles.read().await;
            let mut result: Vec<Candle> = Vec::new();
            
            // Flatten all intervals
            for queue in candles_guard.values() {
                result.extend(queue.iter().cloned());
            }
            return result;
        }
        Vec::new()
    }


    pub async fn get_history(&self, symbol: &str, end_time: u64, limit: usize) -> Vec<Candle> {
        let state = {
            let reg = self.registry.read().await;
            reg.get(symbol).cloned()
        };

        let s = match state {
            Some(v) => v,
            None => return Vec::new(),
        };

        let candles_guard = s.candles.read().await;
        let mut result: Vec<Candle> = Vec::new();

        for queue in candles_guard.values() {
            let filtered: Vec<Candle> = queue.iter()
                .filter(|c| c.start_time < end_time)
                .cloned()
                .collect();
            result.extend(filtered);
        }

        result.sort_by_key(|c| c.start_time);
        
        if result.len() > limit {
            result.split_off(result.len() - limit)
        } else {
            result
        }
    }


    pub async fn request_ingestion(&self, symbol: String) -> bool {
        let mut active_guard = self.active_ingestions.write().await;
        active_guard.insert(symbol)
    }
}