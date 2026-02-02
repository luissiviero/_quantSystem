// File: src/context/WebSocketContext.jsx
import React, { createContext, useEffect } from 'react';
import { binanceStream } from '../services/BinanceStream';

export const WebSocketContext = createContext(null);

export const WebSocketProvider = ({ children }) => {
  useEffect(() => {
    // Start the singleton connection when the app boots
    binanceStream.connect();
  }, []);

  return (
    <WebSocketContext.Provider value={binanceStream}>
      {children}
    </WebSocketContext.Provider>
  );
};