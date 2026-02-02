// File: src/components/TradesComponent.jsx
import React from 'react';
import { useTrades } from '../hooks/useBinanceData';

const TradesComponent = React.memo(() => {
  const trades = useTrades('BTCUSDC');

  return (
    <div className="panel-content" style={{ padding: '8px', display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '10px', color: '#848e9c', marginBottom: '5px', flexShrink: 0 }}>
        <span>Price(USDT)</span><span>Amount(BTC)</span><span>Time</span>
      </div>
      
      {/* Scrollable Container */}
      <div style={{ overflowY: 'auto', flex: 1, minHeight: 0 }}>
        {trades.map(trade => (
          <div key={trade.id} style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', marginBottom: '2px' }}>
            <span className={trade.isMaker ? 'text-sell' : 'text-buy'}>{trade.price}</span>
            <span className="text-white">{trade.amount}</span>
            <span className="text-muted">{trade.time}</span>
          </div>
        ))}
      </div>
    </div>
  );
});

export default TradesComponent;