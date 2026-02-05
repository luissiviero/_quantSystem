// @file: useTradeStore.ts
// @description: Specialized store for managing Trade history.
// @author: v5 helper

import { create } from 'zustand';
import { Trade } from '../models/types';

//
// CONSTANTS
//

const MAX_TRADES: number = 50;

//
// INTERFACES
//

interface TradeState {
  recentTrades: Trade[];
  latestTrade: Trade | null;

  // Actions
  addTrade: (trade: Trade) => void;
}

//
// STORE IMPLEMENTATION
//

export const useTradeStore = create<TradeState>((set) => ({
  // #1. Initial State
  recentTrades: [],
  latestTrade: null,

  // #2. Actions
  addTrade: (trade: Trade) => set((state) => {
    // #1. Prepend new trade
    const updatedTrades: Trade[] = [trade, ...state.recentTrades];
    
    // #2. Enforce size limit
    const cappedTrades: Trade[] = updatedTrades.length > MAX_TRADES
      ? updatedTrades.slice(0, MAX_TRADES)
      : updatedTrades;

    return { 
      recentTrades: cappedTrades,
      latestTrade: trade 
    };
  }),
}));