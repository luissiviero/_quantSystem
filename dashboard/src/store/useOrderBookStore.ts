// @file: useOrderBookStore.ts
// @description: Specialized store for managing Order Book state.
// @author: v5 helper

import { create } from 'zustand';
import { OrderBook } from '../models/types';

//
// INTERFACES
//

interface OrderBookState {
  orderBook: OrderBook | null;
  
  // Actions
  setOrderBook: (book: OrderBook) => void;
}

//
// STORE IMPLEMENTATION
//

export const useOrderBookStore = create<OrderBookState>((set) => ({
  // #1. Initial State
  orderBook: null,

  // #2. State Setters
  setOrderBook: (book: OrderBook) => set({ orderBook: book }),
}));