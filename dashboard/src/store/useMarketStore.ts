// @file: useMarketStore.ts
// @description: Global state management with strict typing and isolated updates.
// @author: v5 helper

import { create } from 'zustand';
import { OrderBook, Trade, AggTrade, Candle, MarketData, DataType, Command } from '../models/types';

//
// CONSTANTS
//

const MAX_TRADES: number = 50; 
const WS_URL: string = 'ws://127.0.0.1:8080';
const SYMBOL: string = 'BTCUSDT';

//
// HELPERS
//

// #1. Sanitization Helper
const sanitizeCandle = (c: Candle): Candle => ({
  ...c,
  start_time: Number(c.start_time),
  open: Number(c.open),
  high: Number(c.high),
  low: Number(c.low),
  close: Number(c.close),
  volume: Number(c.volume),
});

// #2. Channel Helper
const getChannel = (timeframe: string): string => {
    return `${SYMBOL}_${timeframe}`;
};

// keep socket external
let socket: WebSocket | null = null;

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
  activeTimeframe: string; 
  
  // Actions
  setOrderBook: (book: OrderBook) => void;
  addTrade: (trade: Trade) => void;
  addCandle: (candle: Candle) => void;
  prependHistory: (history: Candle[]) => void; 
  setConnected: (status: boolean) => void;
  setLoadingHistory: (status: boolean) => void;
  setTimeframe: (tf: string) => void; 
  
  // Connection Logic
  connect: () => void;
  disconnect: () => void;
}

//
// STORE IMPLEMENTATION
//

export const useMarketStore = create<MarketState>((set, get) => ({
  // 1. Initial State
  orderBook: null,
  recentTrades: [],
  latestTrade: null,
  candles: [],
  latestCandle: null,
  isConnected: false,
  isLoadingHistory: false,
  activeTimeframe: '1m', // Default

  // 2. State Setters
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

  addCandle: (rawCandle: Candle) => set((state) => {
      // #1. Sanitize
      const candle = sanitizeCandle(rawCandle);
      
      // #2. Verify Interval Match (Ignore stray packets from old subs)
      // FIX: Relaxed check. If interval is missing in payload, assume it's valid for current stream.
      if (candle.interval && candle.interval !== state.activeTimeframe) {
          return {};
      }

      const last = state.candles[state.candles.length - 1];
      let newCandles = [...state.candles];
      
      if (last && last.start_time === candle.start_time) {
          newCandles[newCandles.length - 1] = candle;
      } else {
          newCandles.push(candle);
      }

      return { candles: newCandles, latestCandle: candle };
  }),

  prependHistory: (rawHistory: Candle[]) => set((state) => {
      // #1. Sanitize
      const history = rawHistory.map(sanitizeCandle);
      if (history.length === 0) return { isLoadingHistory: false };

      // #2. Merge & Dedup
      const merged = [...history, ...state.candles];
      const uniqueMap = new Map();
      for (const c of merged) {
          uniqueMap.set(c.start_time, c);
      }
      
      const unique = Array.from(uniqueMap.values());
      unique.sort((a, b) => a.start_time - b.start_time);

      return { candles: unique, isLoadingHistory: false };
  }),

  setConnected: (status: boolean) => set({ isConnected: status }),
  setLoadingHistory: (status: boolean) => set({ isLoadingHistory: status }),

  setTimeframe: (tf: string) => {
      const { activeTimeframe, isConnected } = get();
      
      // #1. Avoid redundant switches
      if (tf === activeTimeframe) return;

      console.log(`Switching timeframe: ${activeTimeframe} -> ${tf}`);

      // #2. Handle Subscription Switch if connected
      if (isConnected && socket && socket.readyState === WebSocket.OPEN) {
          // Unsubscribe old
          const unsubMsg: Command = {
              action: 'unsubscribe',
              channel: getChannel(activeTimeframe)
          };
          socket.send(JSON.stringify(unsubMsg));

          // Subscribe new
          const subMsg: Command = {
              action: 'subscribe',
              channel: getChannel(tf),
              // Ensure we request the specific interval needed
              config: {
                raw_trades: true,
                agg_trades: true,
                order_book: true,
                kline_intervals: [tf] // Request only the active timeframe
              }
          };
          socket.send(JSON.stringify(subMsg));

           // Request History for new timeframe
           const histMsg: Command = {
              action: 'fetch_history',
              channel: getChannel(tf)
          };
          socket.send(JSON.stringify(histMsg));
      }

      // #3. Clear Data & Update State
      set({ 
          activeTimeframe: tf, 
          candles: [], // Clear old candles immediately
          latestCandle: null,
          isLoadingHistory: true
      });
  },

  // 3. WebSocket Implementation
  connect: () => {
    if (socket && (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING)) {
        return;
    }

    try {
        socket = new WebSocket(WS_URL);

        socket.onopen = () => {
            console.log("WS: Connected");
            set({ isConnected: true });
            
            // #1. Get current timeframe to subscribe
            const { activeTimeframe } = get();
            const channel = getChannel(activeTimeframe);

            // #2. Subscribe with explicit Config
            const subMsg: Command = { 
                action: 'subscribe', 
                channel,
                config: {
                    raw_trades: true,
                    agg_trades: true,
                    order_book: true,
                    kline_intervals: [activeTimeframe] // Request active + others if needed
                }
            };
            socket?.send(JSON.stringify(subMsg));
            console.log(`WS: Subscribed to ${channel}`);

            // #3. Fetch History
            const histMsg: Command = { action: 'fetch_history', channel };
            socket?.send(JSON.stringify(histMsg));
        };

        socket.onclose = () => {
            console.log("WS: Disconnected");
            set({ isConnected: false });
            socket = null;
        };

        socket.onerror = (err) => {
            console.error("WS: Error", err);
        };

        socket.onmessage = (event: MessageEvent) => {
            try {
                const message: MarketData = JSON.parse(event.data);
                switch (message.type) {
                    case DataType.OrderBook:
                        get().setOrderBook(message.data as OrderBook);
                        break;
                    case DataType.Trade:
                        get().addTrade(message.data as Trade);
                        break;
                    case DataType.AggTrade:
                        // Cast AggTrade to Trade for simple visualization
                        // since AggTrade structure is compatible with Trade for list display
                        get().addTrade(message.data as unknown as Trade);
                        break;
                    case DataType.Candle:
                        get().addCandle(message.data as Candle);
                        break;
                    case DataType.HistoricalCandles:
                        get().prependHistory(message.data as Candle[]);
                        break;
                    default:
                        break;
                }
            } catch (e) {
                console.error("Failed to parse message", e);
            }
        };

    } catch (e) {
        console.error("Connection initiation failed", e);
        set({ isConnected: false });
    }
  },

  disconnect: () => {
      if (socket) {
          const { activeTimeframe } = get();
          const unsubMsg: Command = { action: 'unsubscribe', channel: getChannel(activeTimeframe) };
          if (socket.readyState === WebSocket.OPEN) {
             socket.send(JSON.stringify(unsubMsg));
          }
          socket.close();
          socket = null;
      }
      set({ isConnected: false });
  }
}));