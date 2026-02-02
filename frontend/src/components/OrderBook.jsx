// File: src/components/OrderBook.jsx
import React from 'react';
import { useOrderBook } from '../hooks/useBinanceData';

const OrderBook = React.memo(() => {
  const { asks, bids } = useOrderBook('BTCUSDC');

  const row = (item, colorClass) => (
    <div key={item[0]} style={{ display: 'flex', justifyContent: 'space-between', padding: '1px 0', fontSize: '11px', lineHeight: '16px' }}>
      <span className={colorClass}>{parseFloat(item[0]).toFixed(1)}</span>
      <span className="text-white">{parseFloat(item[1]).toFixed(3)}</span>
      <span className="text-white" style={{ textAlign: 'right', width: '60px' }}>{parseFloat(item[1]).toFixed(3)}</span>
    </div>
  );

  return (
    <div className="panel-content" style={{ padding: '0 8px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', padding: '8px 0', fontSize: '10px', color: '#848e9c' }}>
        <span>Price(USDT)</span><span>Size(BTC)</span><span style={{textAlign: 'right', width: '60px'}}>Sum(BTC)</span>
      </div>
      <div style={{ flex: 1, overflow: 'hidden', display: 'flex', flexDirection: 'column-reverse' }}>
        {asks.slice(0, 15).map(ask => row(ask, 'text-sell'))}
      </div>
      <div style={{ borderTop: '1px solid #2b3139', borderBottom: '1px solid #2b3139', padding: '8px 0', margin: '4px 0', textAlign: 'center' }}>
         <span style={{ fontSize: '14px', color: '#848e9c' }}>Spread</span>
      </div>
      <div style={{ flex: 1, overflow: 'hidden' }}>
        {bids.slice(0, 15).map(bid => row(bid, 'text-buy'))}
      </div>
    </div>
  );
});

export default OrderBook;