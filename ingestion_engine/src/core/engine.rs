// @file: ingestion_engine/src/core/engine.rs
// @description: Engine updated with bulk historical ingestion (no broadcast).
// @author: LAS.

use std::collections::{HashMap, VecDeque, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use crate::core::models::{
    OrderBook, Trade, AggTrade, Candle, MarketData,
    Ticker, BookTicker, MarkPrice, Liquidation, FundingRate, OpenInterest
};
use crate::core::interfaces::DataProcessor;
use crate::utils::config::AppConfig;


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
    
    // NEW STATE FIELDS
    pub ticker: RwLock<Option<Ticker>>,
    pub book_ticker: RwLock<Option<BookTicker>>,
    pub mark_price: RwLock<Option<MarkPrice>>,
    pub liquidations: RwLock<VecDeque<Liquidation>>, 
    pub funding_rate: RwLock<Option<FundingRate>>,
    pub open_interest: RwLock<Option<OpenInterest>>,
}

impl SymbolState {
    fn new(trade_cap: usize, _candle_cap: usize) -> Self {
        Self {
            order_book: RwLock::new(None),
            trades: RwLock::new(VecDeque::with_capacity(trade_cap)),
            agg_trades: RwLock::new(VecDeque::with_capacity(trade_cap)),
            candles: RwLock::new(HashMap::new()),
            
            // Init new fields
            ticker: RwLock::new(None),
            book_ticker: RwLock::new(None),
            mark_price: RwLock::new(None),
            liquidations: RwLock::new(VecDeque::with_capacity(trade_cap)), 
            funding_rate: RwLock::new(None),
            open_interest: RwLock::new(None),
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
    // Config Limits
    pub trade_limit: usize,
    pub candle_limit: usize,
}


impl Engine {
    //
    // INITIALIZATION
    //

    pub fn new(config: &AppConfig) -> Self {
        let (tx, _rx) = broadcast::channel(config.broadcast_buffer_size); 

        Engine {
            registry: Arc::new(RwLock::new(HashMap::new())),
            processors: Arc::new(RwLock::new(Vec::new())),
            active_ingestions: Arc::new(RwLock::new(HashSet::new())),
            tx,
            trade_limit: config.trade_history_limit,
            candle_limit: config.candle_history_limit,
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
        {
            let reg = self.registry.read().await;
            if let Some(state) = reg.get(symbol) {
                return state.clone();
            }
        }

        let mut reg = self.registry.write().await;
        let t_cap = self.trade_limit;
        let c_cap = self.candle_limit;

        reg.entry(symbol.to_string())
            .or_insert_with(|| Arc::new(SymbolState::new(t_cap, c_cap)))
            .clone()
    }


    //
    // EXISTING LOGIC
    //
    
    pub async fn update_order_book(&self, symbol: String, book: OrderBook) {
        let state = self.get_or_create_symbol(&symbol).await;
        {
            let mut book_guard = state.order_book.write().await;
            *book_guard = Some(book.clone());
        }
        self.broadcast_data(MarketData::OrderBook(book)).await;
    }

    pub async fn add_trade(&self, symbol: String, trade: Trade) {
        let state = self.get_or_create_symbol(&symbol).await;
        {
            let mut trades_guard = state.trades.write().await;
            if trades_guard.len() >= self.trade_limit {
                trades_guard.pop_front();
            }
            trades_guard.push_back(trade.clone());
        }
        self.broadcast_data(MarketData::Trade(trade)).await;
    }
    
    pub async fn add_agg_trade(&self, symbol: String, trade: AggTrade) {
        let state = self.get_or_create_symbol(&symbol).await;
        {
            let mut trades_guard = state.agg_trades.write().await;
            if trades_guard.len() >= self.trade_limit {
                trades_guard.pop_front();
            }
            trades_guard.push_back(trade.clone());
        }
        self.broadcast_data(MarketData::AggTrade(trade)).await;
    }

    pub async fn add_candle(&self, symbol: String, candle: Candle) {
        let state = self.get_or_create_symbol(&symbol).await;
        {
            let mut candles_map = state.candles.write().await;
            let queue = candles_map.entry(candle.interval.clone())
                .or_insert_with(|| VecDeque::with_capacity(self.candle_limit));
            if queue.len() >= self.candle_limit {
                queue.pop_front();
            }
            queue.push_back(candle.clone());
        }
        self.broadcast_data(MarketData::Candle(candle)).await;
    }

    //
    // NEW: HISTORICAL INGESTION (No Broadcast)
    //
    
    pub async fn load_historical_candles(&self, symbol: String, candles: Vec<Candle>) {
        if candles.is_empty() { return; }
        
        let state = self.get_or_create_symbol(&symbol).await;
        let mut candles_map = state.candles.write().await;
        
        // #1. Group by interval
        // Ensure we handle mixed intervals if necessary, though usually 
        // history is fetched per interval.
        for candle in candles {
            let queue = candles_map.entry(candle.interval.clone())
                .or_insert_with(|| VecDeque::with_capacity(self.candle_limit));
            
            // #2. Insert logic
            // Since this is history, we might want to ensure order or just push back
            // assuming the API returns sorted data.
            // We verify start time to avoid duplicates if possible, or simple push.
            // For efficiency in this demo, we just push and maintain limit.
            
            queue.push_back(candle);
            
            // #3. Maintain Limit (Basic trimming)
            while queue.len() > self.candle_limit {
                queue.pop_front();
            }
        }
        
        // #4. Sort (Optional but recommended after bulk insert)
        for queue in candles_map.values_mut() {
            queue.make_contiguous().sort_by_key(|c| c.start_time);
        }
    }


    //
    // NEW FEATURE METHODS
    //

    pub async fn update_ticker(&self, symbol: String, ticker: Ticker) {
        let state = self.get_or_create_symbol(&symbol).await;
        {
            let mut ticker_guard = state.ticker.write().await;
            *ticker_guard = Some(ticker.clone());
        }
        self.broadcast_data(MarketData::Ticker(ticker)).await;
    }

    pub async fn update_book_ticker(&self, symbol: String, ticker: BookTicker) {
        let state = self.get_or_create_symbol(&symbol).await;
        {
            let mut bt_guard = state.book_ticker.write().await;
            *bt_guard = Some(ticker.clone());
        }
        self.broadcast_data(MarketData::BookTicker(ticker)).await;
    }

    pub async fn update_mark_price(&self, symbol: String, price: MarkPrice) {
        let state = self.get_or_create_symbol(&symbol).await;
        {
            let mut mp_guard = state.mark_price.write().await;
            *mp_guard = Some(price.clone());
        }
        self.broadcast_data(MarketData::MarkPrice(price)).await;
    }

    pub async fn add_liquidation(&self, symbol: String, liq: Liquidation) {
        let state = self.get_or_create_symbol(&symbol).await;
        {
            let mut liq_guard = state.liquidations.write().await;
            if liq_guard.len() >= self.trade_limit {
                liq_guard.pop_front();
            }
            liq_guard.push_back(liq.clone());
        }
        self.broadcast_data(MarketData::Liquidation(liq)).await;
    }
    
    pub async fn update_funding_rate(&self, symbol: String, rate: FundingRate) {
        let state = self.get_or_create_symbol(&symbol).await;
        {
            let mut fr_guard = state.funding_rate.write().await;
            *fr_guard = Some(rate.clone());
        }
        self.broadcast_data(MarketData::FundingRate(rate)).await;
    }

    pub async fn update_open_interest(&self, symbol: String, oi: OpenInterest) {
         let state = self.get_or_create_symbol(&symbol).await;
        {
            let mut oi_guard = state.open_interest.write().await;
            *oi_guard = Some(oi.clone());
        }
        self.broadcast_data(MarketData::OpenInterest(oi)).await;
    }


    //
    // BROADCAST HELPERS
    //

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
    
    pub async fn get_ticker(&self, symbol: &str) -> Option<Ticker> {
        let state = {
            let reg = self.registry.read().await;
            reg.get(symbol).cloned()
        };
        if let Some(s) = state {
            return s.ticker.read().await.clone();
        }
        None
    }

    pub async fn get_book_ticker(&self, symbol: &str) -> Option<BookTicker> {
        let state = {
            let reg = self.registry.read().await;
            reg.get(symbol).cloned()
        };
        if let Some(s) = state {
            return s.book_ticker.read().await.clone();
        }
        None
    }

    pub async fn get_mark_price(&self, symbol: &str) -> Option<MarkPrice> {
        let state = {
            let reg = self.registry.read().await;
            reg.get(symbol).cloned()
        };
        if let Some(s) = state {
            return s.mark_price.read().await.clone();
        }
        None
    }

    pub async fn get_recent_liquidations(&self, symbol: &str) -> Vec<Liquidation> {
        let state = { let reg = self.registry.read().await; reg.get(symbol).cloned() };
        if let Some(s) = state {
            return s.liquidations.read().await.iter().cloned().collect();
        }
        Vec::new()
    }

    pub async fn get_funding_rate(&self, symbol: &str) -> Option<FundingRate> {
        let state = {
            let reg = self.registry.read().await;
            reg.get(symbol).cloned()
        };
        if let Some(s) = state {
            return s.funding_rate.read().await.clone();
        }
        None
    }

    pub async fn get_open_interest(&self, symbol: &str) -> Option<OpenInterest> {
        let state = {
            let reg = self.registry.read().await;
            reg.get(symbol).cloned()
        };
        if let Some(s) = state {
            return s.open_interest.read().await.clone();
        }
        None
    }

    pub async fn get_recent_trades(&self, symbol: &str) -> Vec<Trade> {
        let state = { let reg = self.registry.read().await; reg.get(symbol).cloned() };
        if let Some(s) = state {
            return s.trades.read().await.iter().cloned().collect();
        }
        Vec::new()
    }
    
    pub async fn get_recent_agg_trades(&self, symbol: &str) -> Vec<AggTrade> {
        let state = { let reg = self.registry.read().await; reg.get(symbol).cloned() };
        if let Some(s) = state {
            return s.agg_trades.read().await.iter().cloned().collect();
        }
        Vec::new()
    }

    pub async fn get_recent_candles(&self, symbol: &str) -> Vec<Candle> {
        let state = { let reg = self.registry.read().await; reg.get(symbol).cloned() };
        if let Some(s) = state {
            let candles_guard = s.candles.read().await;
            let mut result: Vec<Candle> = Vec::new();
            for queue in candles_guard.values() {
                result.extend(queue.iter().cloned());
            }
            return result;
        }
        Vec::new()
    }
    
    pub async fn get_history(&self, symbol: &str, end_time: u64, limit: usize) -> Vec<Candle> {
         let state = { let reg = self.registry.read().await; reg.get(symbol).cloned() };
         if let Some(s) = state {
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
            if result.len() > limit { result.split_off(result.len() - limit) } else { result }
         } else {
             Vec::new()
         }
    }

    pub async fn request_ingestion(&self, symbol: String) -> bool {
        let mut active_guard = self.active_ingestions.write().await;
        active_guard.insert(symbol)
    }
}