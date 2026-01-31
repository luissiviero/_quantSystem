import React, { useEffect, useRef, useState } from 'react';
import { createChart, ColorType, CandlestickSeries } from 'lightweight-charts';

export const ChartComponent = () => {
  const chartContainerRef = useRef();
  const [interval, setInterval] = useState('1m'); 
  const timeframes = ['1m', '5m', '15m', '1h', '4h', '1d'];

  useEffect(() => {
    if (!chartContainerRef.current) return;
    let chart;
    let ws;

    try {
      chart = createChart(chartContainerRef.current, {
        layout: { background: { type: ColorType.Solid, color: '#161a25' }, textColor: '#848e9c' },
        grid: { vertLines: { color: '#2B2B43' }, horzLines: { color: '#2B2B43' } },
        width: chartContainerRef.current.clientWidth,
        height: chartContainerRef.current.clientHeight, 
        timeScale: { timeVisible: true, secondsVisible: false },
      });

      const candleSeries = chart.addSeries(CandlestickSeries, {
        upColor: '#0ecb81', downColor: '#f6465d', borderDownColor: '#f6465d', borderUpColor: '#0ecb81', wickDownColor: '#f6465d', wickUpColor: '#0ecb81',
      });

      // REST Fetch
      fetch(`https://fapi.binance.com/fapi/v1/klines?symbol=BTCUSDC&interval=${interval}&limit=1000`)
        .then(res => res.json())
        .then(data => {
          const candles = data.map(d => ({
            time: d[0] / 1000, open: parseFloat(d[1]), high: parseFloat(d[2]), low: parseFloat(d[3]), close: parseFloat(d[4]),
          }));
          candleSeries.setData(candles);
        });

      // WebSocket
      ws = new WebSocket(`wss://fstream.binance.com/ws/btcusdc@kline_${interval}`);
      ws.onmessage = (event) => {
        const message = JSON.parse(event.data);
        if (message.e === 'kline') {
          const k = message.k;
          candleSeries.update({
            time: k.t / 1000, open: parseFloat(k.o), high: parseFloat(k.h), low: parseFloat(k.l), close: parseFloat(k.c),
          });
        }
      };

      const resizeObserver = new ResizeObserver((entries) => {
          if (entries.length === 0 || !entries[0].target) return;
          const newRect = entries[0].contentRect;
          chart.applyOptions({ height: newRect.height, width: newRect.width });
      });
      resizeObserver.observe(chartContainerRef.current);

      return () => { 
        resizeObserver.disconnect(); 
        if (ws) ws.close();
        if (chart) chart.remove(); 
      };
    } catch (e) { console.log("Chart init error", e); }
  }, [interval]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', width: '100%', backgroundColor: '#161a25' }}>
      <div style={{ height: '30px', display: 'flex', alignItems: 'center', gap: '8px', padding: '0 10px', borderBottom: '1px solid #2B2B43' }}>
        <span style={{ fontSize: '12px', color: '#848e9c', marginRight: '5px' }}>Time</span>
        {timeframes.map((tf) => (
          <button
            key={tf}
            onClick={() => setInterval(tf)}
            style={{
              background: interval === tf ? '#2b3139' : 'transparent',
              color: interval === tf ? '#f0b90b' : '#848e9c',
              border: 'none', cursor: 'pointer', fontSize: '12px', padding: '2px 6px', borderRadius: '2px', fontWeight: interval === tf ? 'bold' : 'normal'
            }}
          >
            {tf}
          </button>
        ))}
      </div>
      <div ref={chartContainerRef} style={{ flex: 1, position: 'relative', width: '100%' }} />
    </div>
  );
};