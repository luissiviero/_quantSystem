// @file: engine.rs
// @description: Core engine logic for managing market state and broadcasting events.
// @author: v5 helper

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use crate::models::{OrderBook, Trade, MarketData};

//
// TYPE DEFINITIONS
//

pub type SharedOrderBook = Arc<RwLock<HashMap<String, OrderBook>>>;
pub type SharedTrades = Arc<RwLock<HashMap<String, Vec<Trade>>>>;

//
// ENGINE STRUCT
//

#[derive(Clone)]
pub struct Engine {
    pub order_books: SharedOrderBook,
    pub recent_trades: SharedTrades,
    // 1. Broadcast channel for real-time updates
    pub tx: broadcast::Sender<MarketData>, 
}

impl Engine {
    //
    // INITIALIZATION
    //

    pub fn new() -> Self {
        let books: SharedOrderBook = Arc::new(RwLock::new(HashMap::new()));
        let trades: SharedTrades = Arc::new(RwLock::new(HashMap::new()));
        
        // 2. Create channel with capacity for 100 buffered messages
        let (tx, _rx) = broadcast::channel(100);

        Engine {
            order_books: books,
            recent_trades: trades,
            tx,
        }
    }

    //
    // STATE MANAGEMENT LOGIC
    //

    pub async fn update_order_book(&self, symbol: String, book: OrderBook) {
        // 1. Update internal state
        let mut books_guard = self.order_books.write().await;
        books_guard.insert(symbol.clone(), book.clone());
        
        // 2. Broadcast event to WebSocket server
        // We ignore errors if no clients are connected (send returns error in that case)
        let _ = self.tx.send(MarketData::OrderBook(book));
    }

    pub async fn add_trade(&self, symbol: String, trade: Trade) {
        // 1. Update internal state
        let mut trades_guard = self.recent_trades.write().await;
        let trades_vec = trades_guard.entry(symbol).or_insert_with(Vec::new);
        
        trades_vec.push(trade.clone());
        
        // 2. Prune old trades
        if trades_vec.len() > 50 {
            trades_vec.remove(0);
        }

        // 3. Broadcast event
        let _ = self.tx.send(MarketData::Trade(trade));
    }

    pub async fn get_order_book(&self, symbol: &str) -> Option<OrderBook> {
        let books_guard = self.order_books.read().await;
        books_guard.get(symbol).cloned()
    }
}