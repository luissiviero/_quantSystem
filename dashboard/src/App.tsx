// dashboard/src/App.tsx
import { useEffect } from 'react';
import { useMarketStore } from './store/useMarketStore';
import { DataType } from './models/types';
import PriceChart from './components/PriceChart';
import OrderBook from './components/OrderBook';
import RecentTrades from './components/RecentTrades';

function App() {
  const connect = useMarketStore((state) => state.connect);
  const isConnected = useMarketStore((state) => state.isConnected);
  const subscribe = useMarketStore((state) => state.subscribeToSymbol);

  // Initialize connection
  useEffect(() => {
    connect('ws://localhost:3000');
  }, [connect]);

  // Subscribe to streams when connected
  useEffect(() => {
    if (isConnected) {
      console.log("App: Subscribing to data feeds...");
      subscribe('BTCUSDT', DataType.Trade);
      subscribe('BTCUSDT', DataType.Depth5);
    }
  }, [isConnected, subscribe]);

  return (
    <div className="min-h-screen bg-black text-white p-6">
      <header className="flex justify-between items-center mb-6 border-b border-gray-800 pb-4">
        <h1 className="text-2xl font-bold text-blue-500 tracking-tight">QuantSystem Dashboard</h1>
        <div className="flex items-center gap-4">
          <div className={`flex items-center gap-2 px-3 py-1 rounded-full border ${isConnected ? 'bg-green-900/20 border-green-900 text-green-400' : 'bg-red-900/20 border-red-900 text-red-400'}`}>
            <span className={`w-2 h-2 rounded-full ${isConnected ? 'bg-green-500 animate-pulse' : 'bg-red-500'}`}></span>
            <span className="text-xs font-bold uppercase tracking-wider">{isConnected ? 'Online' : 'Offline'}</span>
          </div>
        </div>
      </header>

      <main className="max-w-[1600px] mx-auto">
        {/* 3-Column Layout */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-4">
          
          {/* Column 1: Order Book (3 cols) */}
          <div className="lg:col-span-3">
            <OrderBook />
          </div>

          {/* Column 2: Main Chart (6 cols) */}
          <div className="lg:col-span-6">
            <PriceChart />
          </div>

          {/* Column 3: Recent Trades (3 cols) */}
          <div className="lg:col-span-3">
            <RecentTrades />
          </div>

        </div>
      </main>
    </div>
  );
}

export default App;