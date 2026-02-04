// @file: types.ts
// @description: Strict TypeScript definitions matching backend Rust structs.
// @author: v5 helper

//
// ENUMS
//

export enum DataType {
  OrderBook = "OrderBook",
  Trade = "Trade",
  AggTrade = "AggTrade",
  Candle = "Candle",
  HistoricalCandles = "HistoricalCandles" // #1. New Type
}

export enum TradeSide {
  Buy = "Buy",
  Sell = "Sell"
}

//
// ENTITIES
//

export interface PriceLevel {
  price: number;
  quantity: number;
}

export interface OrderBook {
  symbol: string;
  bids: PriceLevel[];
  asks: PriceLevel[];
  last_update_id: number;
}

export interface Trade {
  id: number;
  symbol: string;
  price: number;
  quantity: number;
  timestamp_ms: number;
  side: TradeSide;
}

export interface Candle {
  symbol: string;
  interval: string;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  start_time: number;
  close_time: number;
  is_closed: boolean;
}

//
// MESSAGING
//

export interface MarketData {
  type: DataType;
  data: OrderBook | Trade | Candle | Candle[]; // #2. Union Type
}

export interface Command {
  action: 'subscribe' | 'unsubscribe' | 'fetch_history';
  channel: string;
  end_time?: number;
}