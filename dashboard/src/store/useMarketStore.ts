// dashboard/src/store/useMarketStore.ts
import { create } from 'zustand';
import { subscribeWithSelector } from 'zustand/middleware';
import { DataType, OrderBook, Trade, WebSocketMessage } from '../models/types';

interface MarketState {
    // --- State ---
    isConnected: boolean;
    activeSymbol: string;
    lastTrade: Trade | null;
    tradeHistory: Trade[];
    orderBook: OrderBook | null;

    // --- Actions ---
    connect: (url: string) => void;
    subscribeToSymbol: (symbol: string, type: DataType) => void;
}

export const useMarketStore = create<MarketState>()(
    subscribeWithSelector((set, get) => {
        let socket: WebSocket | null = null;
        let reconnectTimeout: number | undefined;

        const connectSocket = (url: string) => {
            if (socket) {
                // If socket exists and is open or connecting, do nothing
                if (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING) {
                    return;
                }
                // If it's closing or closed, ensure we clear it before creating a new one
                socket.close();
            }

            console.log(`Connecting to ${url}...`);
            socket = new WebSocket(url);

            socket.onopen = () => {
                console.log("WebSocket Connected");
                set({ isConnected: true });
                // Clear any pending reconnect attempts
                clearTimeout(reconnectTimeout);
                
                // Resubscribe to current symbol
                const symbol = get().activeSymbol;
                get().subscribeToSymbol(symbol, DataType.Trade);
                get().subscribeToSymbol(symbol, DataType.Depth5);
            };

            socket.onclose = () => {
                console.log("WebSocket Disconnected. Retrying in 3s...");
                set({ isConnected: false });
                socket = null;

                // Auto-reconnect logic
                reconnectTimeout = window.setTimeout(() => {
                    connectSocket(url);
                }, 3000);
            };

            socket.onerror = (err) => {
                console.error("WebSocket Error:", err);
                socket?.close(); // Trigger onclose
            };

            socket.onmessage = (event) => {
                try {
                    const payload: WebSocketMessage = JSON.parse(event.data);
                    
                    if (payload.type === 'Trade') {
                        const trade = payload.data as Trade;
                        set((state) => ({ 
                            lastTrade: trade,
                            tradeHistory: [trade, ...state.tradeHistory].slice(0, 50)
                        }));
                    } else if (payload.type === 'OrderBook') {
                        set({ orderBook: payload.data as OrderBook });
                    }
                } catch (e) {
                    console.error("Failed to parse WS message", e);
                }
            };
        };

        return {
            isConnected: false,
            activeSymbol: 'BTCUSDT',
            lastTrade: null,
            tradeHistory: [],
            orderBook: null,

            connect: (url: string) => {
                connectSocket(url);
            },

            subscribeToSymbol: (symbol: string, type: DataType) => {
                if (socket?.readyState === WebSocket.OPEN) {
                    const payload = {
                        action: "subscribe",
                        symbol: symbol,
                        dataType: type
                    };
                    socket.send(JSON.stringify(payload));
                    // console.log("Sent subscription:", payload); // Optional logging
                    
                    if (get().activeSymbol !== symbol) {
                         set({ activeSymbol: symbol, tradeHistory: [], orderBook: null });
                    }
                } else {
                    // console.warn("Socket not open, cannot subscribe yet.");
                }
            }
        };
    })
);