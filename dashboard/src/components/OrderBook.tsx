// @file: OrderBook.tsx
// @description: Displays the order book bids and asks visually.
// @author: v5 helper

import React from 'react';
import { useMarketStore } from '../store/useMarketStore';
import { PriceLevel } from '../models/types';

//
// COMPONENT LOGIC
//

const OrderBook: React.FC = () => {
  // 1. Select data from store
  const { orderBook } = useMarketStore();

  if (!orderBook) {
    return (
      <div className="p-4 bg-gray-800 rounded-lg shadow-lg">
        <h2 className="text-xl font-bold mb-4 text-white">Order Book</h2>
        <p className="text-gray-400">Waiting for data...</p>
      </div>
    );
  }

  // 2. Render helper for rows
  const renderRow = (level: PriceLevel, type: 'bid' | 'ask', index: number) => {
    // Explicit typing for styles
    const textColor: string = type === 'bid' ? 'text-green-400' : 'text-red-400';
    
    return (
      <div key={`${type}-${index}`} className="flex justify-between text-sm py-1 border-b border-gray-700 last:border-0">
        <span className={`${textColor} font-mono`}>{level.price.toFixed(2)}</span>
        <span className="text-gray-300 font-mono">{level.quantity.toFixed(5)}</span>
      </div>
    );
  };

  // 3. Main Render
  return (
    <div className="p-4 bg-gray-800 rounded-lg shadow-lg border border-gray-700 h-full">
      <h2 className="text-xl font-bold mb-4 text-white border-b border-gray-600 pb-2">
        Order Book <span className="text-sm text-gray-500">({orderBook.symbol})</span>
      </h2>
      
      <div className="grid grid-cols-2 gap-4">
        {/* BIDS COLUMN */}
        <div>
          <h3 className="text-green-500 font-semibold mb-2 text-center">Bids</h3>
          <div className="space-y-0.5">
            {orderBook.bids.slice(0, 10).map((level: PriceLevel, i: number) => renderRow(level, 'bid', i))}
          </div>
        </div>

        {/* ASKS COLUMN */}
        <div>
          <h3 className="text-red-500 font-semibold mb-2 text-center">Asks</h3>
          <div className="space-y-0.5">
            {orderBook.asks.slice(0, 10).map((level: PriceLevel, i: number) => renderRow(level, 'ask', i))}
          </div>
        </div>
      </div>
    </div>
  );
};

export default OrderBook;