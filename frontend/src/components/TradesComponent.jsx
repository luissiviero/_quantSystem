import React, { useEffect, useRef, useState } from 'react';

const TradesComponent = React.memo(() => {
  const [trades, setTrades] = useState([]);
  const bufferRef = useRef([]); 

  useEffect(() => {
    const ws = new WebSocket('wss://fstream.binance.com/ws/btcusdc@aggTrade');
    
    // 1. FAST: Receive data and push to array (Sync operation)
    ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      const newTrade = {
        id: data.a,
        price: parseFloat(data.p).toFixed(1),
        amount: parseFloat(data.q).toFixed(3),
        time: new Date(data.T).toLocaleTimeString('en-US', { hour12: false }),
        isMaker: data.m
      };
      
      bufferRef.current.unshift(newTrade);
      // Safety Cap: Keep buffer small
      if (bufferRef.current.length > 50) bufferRef.current.length = 50;
    };

    // 2. SLOW: Update UI every 200ms (5 FPS)
    const interval = setInterval(() => {
      if (bufferRef.current.length > 0) {
        setTrades(prev => {
          const combined = [...bufferRef.current, ...prev].slice(0, 50);
          bufferRef.current = []; // Clear buffer
          return combined;
        });
      }
    }, 200);

    return () => { 
      clearInterval(interval);
      ws.close(); 
    };
  }, []);

  return (
    <div className="panel-content" style={{ padding: '8px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '10px', color: '#848e9c', marginBottom: '5px' }}>
        <span>Price(USDT)</span><span>Amount(BTC)</span><span>Time</span>
      </div>
      <div style={{ overflowY: 'hidden', height: '100%' }}>
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