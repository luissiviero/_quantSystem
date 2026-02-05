// @file: useConnectionStore.ts
// @description: Central Controller for WebSocket management. Dispatches data to other stores.
// @author: v5 helper

import { create } from 'zustand';
import { OrderBook, Trade, Candle, MarketData, DataType, Command } from '../models/types';

// Import sibling stores to dispatch updates
import { useOrderBookStore } from './useOrderBookStore';
import { useTradeStore } from './useTradeStore';
import { useCandleStore } from './useCandleStore';

//
// CONSTANTS
//

const WS_URL: string = 'ws://127.0.0.1:8080';
const SYMBOL: string = 'BTCUSDT';

//
// HELPERS
//

const getChannel = (timeframe: string): string => {
  return `${SYMBOL}_${timeframe}`;
};

// Keep socket instance outside store to prevent reactivity loops
let socket: WebSocket | null = null;

//
// INTERFACES
//

interface ConnectionState {
  isConnected: boolean;

  // Actions
  connect: () => void;
  disconnect: () => void;
  switchTimeframe: (newTimeframe: string) => void;
}

//
// STORE IMPLEMENTATION
//

export const useConnectionStore = create<ConnectionState>((set, get) => ({
  // #1. Initial State
  isConnected: false,

  // #2. Connection Logic
  connect: () => {
    if (socket && (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING)) {
      return;
    }

    try {
      socket = new WebSocket(WS_URL);

      socket.onopen = () => {
        console.log("WS: Connected");
        set({ isConnected: true });

        // #1. Get current config from CandleStore
        const { activeTimeframe } = useCandleStore.getState();
        const channel = getChannel(activeTimeframe);

        // #2. Subscribe
        const subMsg: Command = {
          action: 'subscribe',
          channel,
          config: {
            raw_trades: true,
            agg_trades: true,
            order_book: true,
            kline_intervals: [activeTimeframe]
          }
        };
        socket?.send(JSON.stringify(subMsg));

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

      // #3. Central Dispatcher
      socket.onmessage = (event: MessageEvent) => {
        try {
          const message: MarketData = JSON.parse(event.data);
          
          switch (message.type) {
            case DataType.OrderBook:
              useOrderBookStore.getState().setOrderBook(message.data as OrderBook);
              break;
            
            case DataType.Trade:
              useTradeStore.getState().addTrade(message.data as Trade);
              break;
            
            case DataType.AggTrade:
              // Normalized to Trade
              useTradeStore.getState().addTrade(message.data as unknown as Trade);
              break;
            
            case DataType.Candle:
              useCandleStore.getState().addCandle(message.data as Candle);
              break;
            
            case DataType.HistoricalCandles:
              useCandleStore.getState().prependHistory(message.data as Candle[]);
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
      // Unsubscribe current channel before closing
      const { activeTimeframe } = useCandleStore.getState();
      const unsubMsg: Command = { action: 'unsubscribe', channel: getChannel(activeTimeframe) };
      
      if (socket.readyState === WebSocket.OPEN) {
        socket.send(JSON.stringify(unsubMsg));
      }
      
      socket.close();
      socket = null;
    }
    set({ isConnected: false });
  },

  // #3. Orchestration Action
  switchTimeframe: (newTimeframe: string) => {
    const { activeTimeframe } = useCandleStore.getState();
    const { isConnected } = get();

    // 1. Avoid redundant calls
    if (newTimeframe === activeTimeframe) return;

    console.log(`Switching timeframe: ${activeTimeframe} -> ${newTimeframe}`);

    // 2. Handle Socket Operations
    if (isConnected && socket && socket.readyState === WebSocket.OPEN) {
      // Unsubscribe Old
      const unsubMsg: Command = {
        action: 'unsubscribe',
        channel: getChannel(activeTimeframe)
      };
      socket.send(JSON.stringify(unsubMsg));

      // Subscribe New
      const subMsg: Command = {
        action: 'subscribe',
        channel: getChannel(newTimeframe),
        config: {
          raw_trades: true,
          agg_trades: true,
          order_book: true,
          kline_intervals: [newTimeframe]
        }
      };
      socket.send(JSON.stringify(subMsg));

      // Request History
      const histMsg: Command = {
        action: 'fetch_history',
        channel: getChannel(newTimeframe)
      };
      socket.send(JSON.stringify(histMsg));
    }

    // 3. Update Candle Store State
    useCandleStore.getState().resetCandles();
    useCandleStore.getState().setLoadingHistory(true);
    useCandleStore.getState().setTimeframe(newTimeframe);
  }
}));