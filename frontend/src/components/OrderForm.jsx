import React from 'react';

export const OrderForm = () => {
  const inputStyle = { width: '100%', background: '#2b3139', border: 'none', borderRadius: '4px', height: '36px', color: 'white', padding: '0 8px', textAlign: 'right', fontSize: '12px', outline: 'none' };
  
  return (
    <div className="panel-content" style={{ padding: '12px' }}>
      <div style={{ display: 'flex', gap: '15px', fontSize: '12px', color: '#848e9c', marginBottom: '15px' }}>
         <span style={{ color: '#f0b90b', fontWeight: 'bold' }}>Limit</span><span>Market</span><span>Stop Limit</span>
      </div>
      <div style={{ marginBottom: '12px' }}>
         <span style={{ fontSize: '12px', color: '#848e9c', marginBottom: '4px', display: 'block' }}>Price</span>
         <div style={{position: 'relative'}}>
             <input style={inputStyle} defaultValue="84,240.3" />
             <span style={{position: 'absolute', left: '8px', top: '10px', fontSize: '11px', color: '#848e9c'}}>USDT</span>
         </div>
      </div>
      <div style={{ marginBottom: '12px' }}>
         <span style={{ fontSize: '12px', color: '#848e9c', marginBottom: '4px', display: 'block' }}>Size</span>
         <div style={{position: 'relative'}}>
             <input style={inputStyle} placeholder="Size" />
             <span style={{position: 'absolute', left: '8px', top: '10px', fontSize: '11px', color: '#848e9c'}}>BTC</span>
         </div>
      </div>
      <button style={{ width: '100%', background: '#fcd535', color: 'black', border: 'none', borderRadius: '4px', height: '40px', fontWeight: 'bold', fontSize: '14px', cursor: 'pointer', marginTop: '15px' }}>Register Now</button>
    </div>
  );
}