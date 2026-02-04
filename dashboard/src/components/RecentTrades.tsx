// @file: RecentTrades.tsx
// @description: Displays a list of the most recent trades (optimized).
// @author: v5 helper

import React from 'react';
import { useMarketStore } from '../store/useMarketStore';
import { Trade, TradeSide } from '../models/types';

//
// COMPONENT LOGIC
//

const RecentTrades: React.FC = () => {
  const { recentTrades } = useMarketStore();

  const formatTime = (timestamp: number): string => {
    return new Date(timestamp).toLocaleTimeString();
  };

  return (
    <div className="bg-gray-800 rounded-lg p-4 border border-gray-700 h-full overflow-hidden flex flex-col">
      <h2 className="text-lg font-bold text-white mb-3 border-b border-gray-600 pb-2">
        Recent Trades
      </h2>
      
      {/* 2. Header Row */}
      <div className="grid grid-cols-3 text-xs text-gray-400 font-semibold mb-2">
        <span>Price (USDT)</span>
        <span className="text-right">Qty (BTC)</span>
        <span className="text-right">Time</span>
      </div>

      {/* 3. Trade List */}
      <div className="overflow-y-auto flex-1 space-y-1 custom-scrollbar">
        {recentTrades.length === 0 ? (
          <p className="text-gray-500 text-center text-sm mt-4">Waiting for trades...</p>
        ) : (
          // FIX: Slice the large history array to only show the last 50 items in the UI list
          recentTrades.slice(0, 50).map((trade: Trade, index: number) => {
            // Guard clause for safety in case of bad data
            if (!trade) return null;

            // FIX: Use 'side' property instead of 'is_buyer_maker'
            const isSell: boolean = trade.side === TradeSide.Sell;
            const colorClass: string = isSell ? 'text-red-400' : 'text-green-400';

            return (
              <div key={`${trade.id || index}`} className="grid grid-cols-3 text-sm hover:bg-gray-700/50 rounded p-1">
                <span className={`${colorClass} font-mono`}>
                  {trade.price.toFixed(2)}
                </span>
                <span className="text-gray-300 text-right font-mono">
                  {trade.quantity.toFixed(5)}
                </span>
                <span className="text-gray-500 text-right text-xs pt-0.5">
                  {formatTime(trade.timestamp_ms)}
                </span>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};

export default RecentTrades;