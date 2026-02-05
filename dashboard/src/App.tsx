/**
 * @file: App.tsx
 * @description: Main application entry point. Handles WebSocket connection via ConnectionStore
 * and layout of the dashboard.
 * @tags: #frontend #entrypoint #websocket #layout
 */

import { useState, useEffect, useRef } from 'react';
import OrderBook from './components/OrderBook';
import PriceChart from './components/PriceChart';
import RecentTrades from './components/RecentTrades';

// Use the new Connection Store
import { useConnectionStore } from './store/useConnectionStore';

//
// TYPES
//

type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error';

//
// MAIN COMPONENT
//

export default function App() {
  //
  // STATE
  //

  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>('disconnected');
  
  // Destructure from Connection Store
  const { connect, disconnect, isConnected } = useConnectionStore();
  
  const reconnectTimeout = useRef<number | null>(null);

  //
  // SIDE EFFECTS
  //

  useEffect(() => {
    let didInitiate = false;
    let mountTimer: number;

    const initiateConnection = () => {
      setConnectionStatus('connecting');
      console.log('Attempting connection to Ingestion Engine...');
      
      try {
        connect();
        didInitiate = true;
      } catch (error) {
        console.error('Connection failed', error);
        setConnectionStatus('error');
      }
    };

    // Debounce for Strict Mode
    mountTimer = window.setTimeout(initiateConnection, 50);

    return () => {
      window.clearTimeout(mountTimer);
      
      if (reconnectTimeout.current !== null) {
        window.clearTimeout(reconnectTimeout.current);
      }
      
      if (didInitiate) {
        disconnect();
      }
    };
  }, [connect, disconnect]);


  // Monitor connection state and trigger reconnection if needed
  useEffect(() => {
    if (isConnected) {
      setConnectionStatus('connected');
      if (reconnectTimeout.current !== null) {
        window.clearTimeout(reconnectTimeout.current);
        reconnectTimeout.current = null;
      }
      return;
    }

    if (!isConnected && connectionStatus !== 'connecting') {
      if (reconnectTimeout.current === null) {
          reconnectTimeout.current = window.setTimeout(() => {
            setConnectionStatus('connecting');
            connect();
            reconnectTimeout.current = null; 
          }, 5000);
      }
    }
  }, [isConnected, connectionStatus, connect]);

  //
  // RENDER
  //

  return (
    <div className="min-h-screen bg-gray-900 text-gray-100 p-4 font-mono">
      {/* HEADER */}
      <header className="mb-6 border-b border-gray-700 pb-4 flex justify-between items-center">
        <div>
          <h1 className="text-2xl font-bold text-blue-400">QUANT SYSTEM v5</h1>
          <p className="text-xs text-gray-500 mt-1">HFT DASHBOARD // PROPRIETARY</p>
        </div>
        
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <span className="text-sm text-gray-400">STATUS:</span>
            <span className={`px-2 py-1 rounded text-xs font-bold ${
              connectionStatus === 'connected' ? 'bg-green-900 text-green-400' : 
              connectionStatus === 'connecting' ? 'bg-yellow-900 text-yellow-400' : 
              'bg-red-900 text-red-400'
            }`}>
              {connectionStatus.toUpperCase()}
            </span>
          </div>
        </div>
      </header>

      {/* MAIN GRID */}
      <main className="grid grid-cols-12 gap-6 h-[calc(100vh-140px)]">
        
        {/* LEFT COLUMN: PRICE CHART */}
        <div className="col-span-12 lg:col-span-8 bg-gray-800 rounded-lg border border-gray-700 overflow-hidden flex flex-col">
          <div className="p-3 bg-gray-800 border-b border-gray-700 flex justify-between items-center">
            <h2 className="text-sm font-semibold text-gray-300">BTC/USDT PRICE ACTION</h2>
          </div>
          <div className="flex-1 relative">
            <PriceChart />
          </div>
        </div>

        {/* RIGHT COLUMN: DATA FEEDS */}
        <div className="col-span-12 lg:col-span-4 flex flex-col gap-6 h-full">
          
          {/* ORDER BOOK */}
          <div className="flex-1 bg-gray-800 rounded-lg border border-gray-700 overflow-hidden flex flex-col">
            <div className="p-3 bg-gray-800 border-b border-gray-700">
              <h2 className="text-sm font-semibold text-gray-300">ORDER BOOK</h2>
            </div>
            <div className="flex-1 overflow-auto">
              <OrderBook />
            </div>
          </div>

          {/* RECENT TRADES */}
          <div className="flex-1 bg-gray-800 rounded-lg border border-gray-700 overflow-hidden flex flex-col">
            <div className="p-3 bg-gray-800 border-b border-gray-700">
              <h2 className="text-sm font-semibold text-gray-300">RECENT TRADES</h2>
            </div>
            <div className="flex-1 overflow-auto">
              <RecentTrades />
            </div>
          </div>

        </div>
      </main>
    </div>
  );
}