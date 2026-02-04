// @file: useMarketStore.ts
// @description: Global state management with strict typing and isolated updates.
// @author: v5 helper

import { create } from 'zustand';
import { OrderBook, Trade, Candle } from '../models/types';

//
// CONSTANTS
//

const MAX_TRADES: number = 10000;

//
// INTERFACES
//

interface MarketState {
  orderBook: OrderBook | null;
  recentTrades: Trade[];
  latestTrade: Trade | null;
  candles: Candle[];            
  latestCandle: Candle | null; 
  isConnected: boolean;
  isLoadingHistory: boolean;    
  
  setOrderBook: (book: OrderBook) => void;
  addTrade: (trade: Trade) => void;
  addCandle: (candle: Candle) => void;
  prependHistory: (history: Candle[]) => void; 
  setConnected: (status: boolean) => void;
  setLoadingHistory: (status: boolean) => void;
}

//
// STORE IMPLEMENTATION
//

export const useMarketStore = create<MarketState>((set) => ({
  // 1. Initial State
  orderBook: null,
  recentTrades: [],
  latestTrade: null,
  candles: [],
  latestCandle: null,
  isConnected: false,
  isLoadingHistory: false,

  // 2. Actions
  setOrderBook: (book: OrderBook) => set({ orderBook: book }),

  addTrade: (trade: Trade) => set((state: MarketState) => {
    const updatedTrades: Trade[] = [trade, ...state.recentTrades];
    const cappedTrades: Trade[] = updatedTrades.length > MAX_TRADES
      ? updatedTrades.slice(0, MAX_TRADES)
      : updatedTrades;
    return { 
      recentTrades: cappedTrades,
      latestTrade: trade 
    };
  }),

  // Handle live incoming candle updates
  addCandle: (candle: Candle) => set((state) => {
      // Logic: Update the last candle if it's the same time, or append if new
      const last = state.candles[state.candles.length - 1];
      let newCandles = [...state.candles];
      
      if (last && last.start_time === candle.start_time) {
          newCandles[newCandles.length - 1] = candle;
      } else {
          newCandles.push(candle);
      }

      return { candles: newCandles, latestCandle: candle };
  }),

  // Handle bulk history load (Infinite Scroll)
  prependHistory: (history: Candle[]) => set((state) => {
      // 1. Merge
      const merged = [...history, ...state.candles];
      
      // 2. Deduplicate
      // Use a Map keyed by start_time. If duplicates exist, later ones overwrite earlier ones.
      const uniqueMap = new Map();
      for (const c of merged) {
          uniqueMap.set(c.start_time, c);
      }
      
      // 3. Flatten
      const unique = Array.from(uniqueMap.values());

      // 4. Strict Sort (CRITICAL FIX)
      // We must guarantee strictly ascending order for Lightweight Charts.
      unique.sort((a, b) => a.start_time - b.start_time);

      return { candles: unique, isLoadingHistory: false };
  }),

  setConnected: (status: boolean) => set({ isConnected: status }),
  setLoadingHistory: (status: boolean) => set({ isLoadingHistory: status }),
}));