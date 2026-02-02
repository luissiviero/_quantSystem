// dashboard/src/models/types.ts

export enum DataType {
    Trade = "Trade",
    Depth5 = "Depth5"
}

export interface Trade {
    symbol: string;
    price: number;
    quantity: number;
    timestamp: number;
}

export interface OrderBook {
    symbol: string;
    bids: [number, number][]; // [price, qty]
    asks: [number, number][];
}

export interface WebSocketMessage {
    type: "Trade" | "OrderBook";
    data: Trade | OrderBook;
}