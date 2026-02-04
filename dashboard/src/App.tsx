// @file: App.tsx
// @description: Main application component with robust WebSocket reconnection logic.
// @author: v5 helper

import React, { useEffect, useRef, useCallback } from 'react';
import { useMarketStore } from './store/useMarketStore';
import OrderBook from './components/OrderBook';
import RecentTrades from './components/RecentTrades';
import PriceChart from './components/PriceChart';
import { MarketData, DataType, Command } from './models/types';

//
// CONSTANTS
//

const WS_URL: string = 'ws://127.0.0.1:8080';
const RECONNECT_DELAY_MS: number = 3000;

//
// COMPONENT LOGIC
//

const App: React.FC = () => {
  // #1. Destructure new actions for Candles and History
  const { 
      setOrderBook, 
      addTrade, 
      addCandle, 
      prependHistory,
      setConnected, 
      isConnected 
  } = useMarketStore();
  
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<number | null>(null);

  // 1. Define Connection Logic
  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN || wsRef.current?.readyState === WebSocket.CONNECTING) {
        return;
    }

    console.log('Attempting connection to Ingestion Engine...');
    const ws: WebSocket = new WebSocket(WS_URL);
    wsRef.current = ws;

    ws.onopen = () => {
      console.log('✅ Connected to Ingestion Engine');
      setConnected(true);
      if (reconnectTimeoutRef.current) {
          clearTimeout(reconnectTimeoutRef.current);
          reconnectTimeoutRef.current = null;
      }
      
      const cmd: string = JSON.stringify({ action: 'subscribe', channel: 'BTCUSDT' });
      ws.send(cmd);
    };

    ws.onclose = () => {
      console.log('❌ Disconnected. Retrying in 3s...');
      setConnected(false);
      wsRef.current = null;
      reconnectTimeoutRef.current = setTimeout(() => {
          connect();
      }, RECONNECT_DELAY_MS);
    };

    ws.onerror = () => { };

    ws.onmessage = (event: MessageEvent) => {
      try {
        const payload: MarketData = JSON.parse(event.data);
        
        // #2. Switch on DataType to handle all incoming message types
        switch (payload.type) {
            case DataType.OrderBook:
                // Cast to any to satisfy TS union discrimination in this simple switch
                setOrderBook(payload.data as any);
                break;
            case DataType.Trade:
                addTrade(payload.data as any);
                break;
            // FIX: Handle Live Candles
            case DataType.Candle:
                addCandle(payload.data as any);
                break;
            // FIX: Handle Bulk History (Infinite Scroll)
            case DataType.HistoricalCandles:
                prependHistory(payload.data as any);
                break;
        }
      } catch (e) {
        // Silently ignore parse errors
      }
    };
  }, [setOrderBook, addTrade, addCandle, prependHistory, setConnected]);

  // 2. Setup Effects
  useEffect(() => {
    const connectionDelay = setTimeout(() => {
      connect();
    }, 100);

    // #3. Setup Listener for outbound commands from Components (e.g. PriceChart)
    const handleWsSend = (e: Event) => {
        const customEvent = e as CustomEvent<Command>;
        if (wsRef.current?.readyState === WebSocket.OPEN) {
            console.log("Sending Command:", customEvent.detail);
            wsRef.current.send(JSON.stringify(customEvent.detail));
        }
    };
    
    window.addEventListener('ws-send', handleWsSend);

    return () => {
      clearTimeout(connectionDelay);
      window.removeEventListener('ws-send', handleWsSend); // Cleanup listener

      if (wsRef.current) {
        wsRef.current.onclose = null; 
        wsRef.current.close();
        wsRef.current = null;
      }
      if (reconnectTimeoutRef.current) {
          clearTimeout(reconnectTimeoutRef.current);
      }
    };
  }, [connect]);

  // 3. Render Layout
  return (
    <div className="min-h-screen bg-gray-900 text-white p-4 lg:p-6 font-sans">
      <header className="mb-6 flex justify-between items-center border-b border-gray-700 pb-4">
        <div>
          <h1 className="text-2xl font-bold text-blue-400 tracking-tight">QuantSystem</h1>
          <p className="text-xs text-gray-400">Institutional Grade Market Data</p>
        </div>
        <div className="flex items-center gap-3 bg-gray-800 px-3 py-1.5 rounded-full border border-gray-700">
          <span className={`h-2.5 w-2.5 rounded-full shadow-glow ${isConnected ? 'bg-green-500 shadow-green-500/50' : 'bg-red-500 shadow-red-500/50'}`}></span>
          <span className="text-sm font-medium text-gray-300">{isConnected ? 'LIVE FEED' : 'RECONNECTING...'}</span>
        </div>
      </header>

      <main className="grid grid-cols-1 lg:grid-cols-12 gap-6 h-[calc(100vh-140px)]">
        {/* Left: Order Book (3 cols) */}
        <div className="lg:col-span-3 h-full overflow-hidden">
           <OrderBook />
        </div>

        {/* Center: Chart (6 cols) */}
        <div className="lg:col-span-6 h-full flex flex-col gap-4 overflow-hidden">
           <PriceChart />
        </div>

        {/* Right: Recent Trades (3 cols) */}
        <div className="lg:col-span-3 h-full overflow-hidden">
           <RecentTrades />
        </div>
      </main>
    </div>
  );
};

export default App;