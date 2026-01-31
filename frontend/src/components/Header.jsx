import React, { useEffect, useState } from 'react';

const Header = React.memo(() => {
  const [price, setPrice] = useState('0.00');
  const [stats, setStats] = useState({ change: '0.00', mark: '0.00', index: '0.00' });

  useEffect(() => {
    // Stream 1: Real-time Price (Fast)
    const wsPrice = new WebSocket('wss://fstream.binance.com/ws/btcusdc@aggTrade');
    wsPrice.onmessage = (event) => {
      const data = JSON.parse(event.data);
      setPrice(parseFloat(data.p).toFixed(1));
    };

    // Stream 2: 24h Stats (Slow - 1s updates)
    const wsStats = new WebSocket('wss://fstream.binance.com/ws/btcusdc@ticker');
    wsStats.onmessage = (event) => {
      const data = JSON.parse(event.data);
      setStats({
        change: parseFloat(data.P).toFixed(2),
        mark: parseFloat(data.c).toFixed(1),
        index: parseFloat(data.c).toFixed(1) 
      });
    };

    return () => { wsPrice.close(); wsStats.close(); };
  }, []);

  const changeColor = parseFloat(stats.change) >= 0 ? '#0ecb81' : '#f6465d';

  return (
    <div style={{ height: '50px', backgroundColor: '#161a25', borderBottom: '1px solid #0b0e11', display: 'flex', alignItems: 'center', padding: '0 16px', justifyContent: 'space-between', flexShrink: 0 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: '24px' }}>
        <div style={{ color: '#f0b90b', fontWeight: 'bold', fontSize: '16px', letterSpacing: '1px' }}>BINANCE <span style={{color: 'white', fontWeight: 'normal'}}>FUTURES</span></div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '15px', fontSize: '12px' }}>
          <div style={{ display: 'flex', flexDirection: 'column' }}>
             <span style={{ fontWeight: 'bold', fontSize: '14px', color: '#eaecef' }}>BTCUSDC <span style={{fontSize: '10px', background: '#2b3139', padding: '1px 3px', borderRadius: '2px', color: '#848e9c'}}>Perp</span></span>
             <span style={{ color: changeColor, textDecoration: 'underline', textDecorationStyle: 'dotted' }}>24h Change</span>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column' }}>
             <span style={{ color: changeColor, fontSize: '16px', fontWeight: 'bold' }}>{price}</span>
             <span style={{ color: '#848e9c', fontSize: '11px' }}>${price}</span>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column' }}>
             <span style={{ color: '#848e9c', fontSize: '11px' }}>Mark</span>
             <span style={{ color: '#eaecef' }}>{stats.mark}</span>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column' }}>
             <span style={{ color: '#f0b90b', fontSize: '11px' }}>Funding</span>
             <span style={{ color: '#eaecef' }}>0.0100% <span style={{color: '#848e9c'}}>02:43:35</span></span>
          </div>
        </div>
      </div>
      <div style={{ display: 'flex', gap: '15px', fontSize: '12px', color: '#848e9c' }}>
        <span style={{cursor: 'pointer'}}>Log In</span>
        <span style={{ background: '#f0b90b', color: 'black', padding: '4px 12px', borderRadius: '4px', fontWeight: 'bold', cursor: 'pointer' }}>Register</span>
      </div>
    </div>
  );
});

export default Header;