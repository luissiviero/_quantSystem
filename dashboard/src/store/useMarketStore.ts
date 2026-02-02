// @file: useMarketStore.ts
// @description: Global state management for market data.
// @author: v5 helper

import { create } from 'zustand';
import { OrderBook, Trade } from '../models/types';

//
// INTERFACES
//

interface MarketState {
  orderBook: OrderBook | null;
  recentTrades: Trade[];
  isConnected: boolean;
  setOrderBook: (book: OrderBook) => void;
  addTrade: (trade: Trade) => void;
  setConnected: (status: boolean) => void;
}

//
// STORE IMPLEMENTATION
//

export const useMarketStore = create<MarketState>((set) => ({
  // 1. Initial State
  orderBook: null,
  recentTrades: [],
  isConnected: false,

  // 2. Actions
  setOrderBook: (book: OrderBook) => set({ orderBook: book }),
  
  addTrade: (trade: Trade) => set((state: MarketState) => {
    // 2a. Prepend new trade
    const updatedTrades: Trade[] = [trade, ...state.recentTrades];
    
    // 2b. Limit array size
    // FIX: Increased from 50 to 2500 to allow chart history to build up.
    // The RecentTrades component will slice this array to avoid UI lag.
    const MAX_TRADES: number = 2500;
    
    if (updatedTrades.length > MAX_TRADES) {
        return { recentTrades: updatedTrades.slice(0, MAX_TRADES) };
    }
    
    return { recentTrades: updatedTrades };
  }),

  setConnected: (status: boolean) => set({ isConnected: status }),
}));