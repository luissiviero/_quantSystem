// dashboard/src/models/types.ts

// Must match Rust 'DataType' enum
export enum DataType {
    Trade = "Trade",
    Depth5 = "Depth5"
}

// Must match Rust 'Trade' struct
export interface Trade {
    symbol: string;
    price: number;
    quantity: number;
    timestamp: number;
}

// Must match Rust 'OrderBook' struct
export interface OrderBook {
    symbol: string;
    bids: [number, number][]; // [price, qty]
    asks: [number, number][];
}

// The wrapper message from Backend
export interface WebSocketMessage {
    type: "Trade" | "OrderBook";
    data: Trade | OrderBook;
}