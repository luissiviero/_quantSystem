// File: src/hooks/useBinanceData.js
import { useEffect, useState, useRef, useContext } from 'react';
import { WebSocketContext } from '../context/WebSocketContext';

// Hook 1: Real-time Price (Ticker)
export const useTicker = (symbol) => {
  const ws = useContext(WebSocketContext);
  const [ticker, setTicker] = useState({ price: '0.00', change: '0.00', mark: '0.00' });

  useEffect(() => {
    const streamName = `${symbol.toLowerCase()}@ticker`;
    
    const handleMsg = (data) => {
      if (!data) return;
      setTicker({
        price: parseFloat(data.c || 0).toFixed(1),
        change: parseFloat(data.P || 0).toFixed(2),
        // Note: 24hrTicker stream gives Last Price (c), not Mark Price. 
        // We use Last Price as a placeholder for Mark to keep it simple.
        mark: parseFloat(data.c || 0).toFixed(1)
      });
    };

    ws.subscribe(streamName, handleMsg);
    return () => ws.unsubscribe(streamName, handleMsg);
  }, [symbol, ws]);

  return ticker;
};

// Hook 2: Order Book
export const useOrderBook = (symbol) => {
  const ws = useContext(WebSocketContext);
  const [orderBook, setOrderBook] = useState({ asks: [], bids: [] });

  useEffect(() => {
    const streamName = `${symbol.toLowerCase()}@depth10@100ms`; 
    
    const handleMsg = (data) => {
      if (!data) return;
      setOrderBook({
        asks: data.a || [],
        bids: data.b || []
      });
    };

    ws.subscribe(streamName, handleMsg);
    return () => ws.unsubscribe(streamName, handleMsg);
  }, [symbol, ws]);

  return orderBook;
};

// Hook 3: Recent Trades
export const useTrades = (symbol) => {
  const ws = useContext(WebSocketContext);
  const [trades, setTrades] = useState([]);
  const buffer = useRef([]);

  useEffect(() => {
    const streamName = `${symbol.toLowerCase()}@aggTrade`;
    
    const handleMsg = (data) => {
      if (!data) return;
       const newTrade = {
        id: data.a, // Aggregate Trade ID
        price: parseFloat(data.p).toFixed(1),
        amount: parseFloat(data.q).toFixed(3),
        time: new Date(data.T).toLocaleTimeString('en-US', { hour12: false }),
        isMaker: data.m
      };
      buffer.current.unshift(newTrade);
      if (buffer.current.length > 50) buffer.current.length = 50;
    };

    ws.subscribe(streamName, handleMsg);
    
    // UI Update Loop (Throttling to prevent React render overload)
    const interval = setInterval(() => {
      if (buffer.current.length > 0) {
        setTrades(prev => {
          const combined = [...buffer.current, ...prev].slice(0, 50);
          buffer.current = [];
          return combined;
        });
      }
    }, 200);

    return () => {
      ws.unsubscribe(streamName, handleMsg);
      clearInterval(interval);
    };
  }, [symbol, ws]);

  return trades;
};