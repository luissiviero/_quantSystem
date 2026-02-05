// @file: useCandleStore.ts
// @description: Specialized store for managing Candle/Kline data and Timeframe state.
// @author: v5 helper

import { create } from 'zustand';
import { Candle } from '../models/types';

//
// HELPERS
//

const sanitizeCandle = (c: Candle): Candle => ({
  ...c,
  start_time: Number(c.start_time),
  open: Number(c.open),
  high: Number(c.high),
  low: Number(c.low),
  close: Number(c.close),
  volume: Number(c.volume),
});

//
// INTERFACES
//

interface CandleState {
  candles: Candle[];
  latestCandle: Candle | null;
  activeTimeframe: string;
  isLoadingHistory: boolean;

  // Actions
  addCandle: (candle: Candle) => void;
  prependHistory: (history: Candle[]) => void;
  setTimeframe: (tf: string) => void;
  setLoadingHistory: (status: boolean) => void;
  resetCandles: () => void;
}

//
// STORE IMPLEMENTATION
//

export const useCandleStore = create<CandleState>((set, get) => ({
  // #1. Initial State
  candles: [],
  latestCandle: null,
  activeTimeframe: '1m',
  isLoadingHistory: false,

  // #2. Actions
  setTimeframe: (tf: string) => set({ activeTimeframe: tf }),
  setLoadingHistory: (status: boolean) => set({ isLoadingHistory: status }),
  resetCandles: () => set({ candles: [], latestCandle: null }),

  addCandle: (rawCandle: Candle) => set((state) => {
    // #1. Sanitize
    const candle = sanitizeCandle(rawCandle);

    // #2. Validate Timeframe
    // If interval is present in payload, match it against active timeframe
    if (candle.interval && candle.interval !== state.activeTimeframe) {
      return {};
    }

    const last = state.candles[state.candles.length - 1];
    let newCandles = [...state.candles];

    // #3. Upsert Logic (Update if same time, Append if new)
    if (last && last.start_time === candle.start_time) {
      newCandles[newCandles.length - 1] = candle;
    } else {
      newCandles.push(candle);
    }

    return { candles: newCandles, latestCandle: candle };
  }),

  prependHistory: (rawHistory: Candle[]) => set((state) => {
    // #1. Sanitize & Empty Check
    const history = rawHistory.map(sanitizeCandle);
    if (history.length === 0) return { isLoadingHistory: false };

    // #2. Merge & Deduplicate
    const merged = [...history, ...state.candles];
    const uniqueMap = new Map();
    for (const c of merged) {
      uniqueMap.set(c.start_time, c);
    }

    // #3. Sort
    const unique = Array.from(uniqueMap.values());
    unique.sort((a, b) => a.start_time - b.start_time);

    return { candles: unique, isLoadingHistory: false };
  }),
}));