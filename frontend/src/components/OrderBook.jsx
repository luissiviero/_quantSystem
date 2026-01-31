import React, { useEffect, useState } from 'react';

const OrderBook = React.memo(() => {
  const [asks, setAsks] = useState([]);
  const [bids, setBids] = useState([]);
  const [lastPrice, setLastPrice] = useState('Loading...');

  useEffect(() => {
    // Optimization: Stream merging
    const ws = new WebSocket('wss://fstream.binance.com/stream?streams=btcusdc@depth10@100ms/btcusdc@aggTrade');
    
    ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      const stream = message.stream;
      const data = message.data;

      if (stream.includes('depth10')) {
        setAsks(data.a || []);
        setBids(data.b || []);
      } else if (stream.includes('aggTrade')) {
        setLastPrice(parseFloat(data.p).toFixed(1));
      }
    };

    return () => { ws.close(); };
  }, []);

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
      <div style={{ borderTop: '1px solid #2b3139', borderBottom: '1px solid #2b3139', padding: '8px 0', margin: '4px 0', display: 'flex', alignItems: 'center', gap: '8px' }}>
         <span style={{ fontSize: '16px', color: '#f0b90b', fontWeight: 'bold' }}>{lastPrice}</span>
      </div>
      <div style={{ flex: 1, overflow: 'hidden' }}>
        {bids.slice(0, 15).map(bid => row(bid, 'text-buy'))}
      </div>
    </div>
  );
});

export default OrderBook;