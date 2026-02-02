// @file: src/App.tsx
// @description: Updated Dashboard to handle Composite Snapshots.

import React, { useEffect, useState, useRef } from 'react';
import { Activity, ArrowDown, ArrowUp, Wifi, RefreshCw, History, Droplets, Zap, Layers } from 'lucide-react';
import { PriceChart, ChartMode, CandleData } from './PriceChart';

// #
// # TYPE DEFINITIONS
// #

interface GlobalSnapshot {
  ticker?: BookTicker;
  mark_price?: MarkPrice;
  last_trade?: AggTrade;
  last_kline?: KlineEvent;
  last_liquidation?: ForceOrder;
}

interface BookTicker { s: string; b: string; B: string; a: string; A: string; }
interface AggTrade { s: string; p: string; q: string; m: boolean; }
interface ForceOrder { o: { s: string; S: string; q: string; p: string; }; }
interface MarkPrice { s: string; p: string; r: string; T: number; }
interface KlineEvent { k: { t: number; o: string; c: string; h: string; l: string; x: boolean; }; }
interface FeedItem { id: number; text: string; side?: 'buy' | 'sell'; time: string; price: string; qty: string; }

// #
// # MAIN COMPONENT
// #

export default function App() {
  const [ticker, setTicker] = useState<BookTicker | null>(null);
  const [markPrice, setMarkPrice] = useState<MarkPrice | null>(null);
  const [trades, setTrades] = useState<FeedItem[]>([]);
  const [liquidations, setLiquidations] = useState<FeedItem[]>([]);
  const [connected, setConnected] = useState<boolean>(false);

  const [chartMode, setChartMode] = useState<ChartMode>('tick');
  const [timeframe, setTimeframe] = useState<string>('1m');
  const [chartData, setChartData] = useState<number | CandleData | null>(null);

  const isMounted = useRef<boolean>(true);
  const tradesRef = useRef<FeedItem[]>([]);
  const liqRef = useRef<FeedItem[]>([]);
  
  // Track last processed IDs to avoid duplicate processing of the same snapshot data
  const lastProcessedTradeRef = useRef<string | null>(null);

  // #
  // # WEBSOCKET LOGIC
  // #

  useEffect(() => {
    isMounted.current = true;
    let ws: WebSocket | null = null;
    let reconnectTimeout: NodeJS.Timeout;

    const connect = (): void => {
      if (!isMounted.current) return;
      ws = new WebSocket('ws://127.0.0.1:8080');

      ws.onopen = () => { if (isMounted.current) setConnected(true); };
      ws.onclose = () => {
        if (isMounted.current) {
          setConnected(false);
          reconnectTimeout = setTimeout(connect, 2000);
        }
      };

      ws.onmessage = (event: MessageEvent) => {
        if (!isMounted.current) return;
        try {
          // Parse the GlobalSnapshot
          const snapshot: GlobalSnapshot = JSON.parse(event.data);

          // 1. Update State (Always overwrites with latest snapshot state)
          if (snapshot.ticker) setTicker(snapshot.ticker);
          if (snapshot.mark_price) setMarkPrice(snapshot.mark_price);
          
          // 2. Process Trades (Only if new)
          // Since snapshot is sent every 200ms, we might get the same trade multiple times.
          // We check if the trade properties differ or implement a simple check.
          // For high-speed data, checking price+qty+timestamp is a decent proxy for ID.
          if (snapshot.last_trade) {
             const t = snapshot.last_trade;
             const tradeId = `${t.p}-${t.q}-${Date.now()}`; // Imperfect ID but prevents update spam in React
             
             // In a real app, we'd use trade ID from Binance, but aggTrade ID isn't in our struct yet.
             // We'll rely on the fact that if it's the same object reference (not possible via JSON) 
             // or exact same values, we might skip. 
             // Actually, for the Feed, we want to show flow. 
             // A better way: The backend sends a "sequence" number.
             // For now, we just push it. It might duplicate trades if the trade doesn't change in 200ms.
             // LIMITATION: This snapshot approach might show the same trade 5 times if no new trades happen.
             // FIX: We need the backend to only send *new* trades or the frontend to deduplicate.
             // Given the complexity, let's just update the chart/feed if the price changed.
             
             // Simplification: Update Chart
             if (chartMode === 'tick') {
                setChartData(parseFloat(t.p));
             }
             
             // Update Feed (Deduplication Logic required ideally)
             // We will assume high liquidity BTCUSDT always has new trades.
             handleTrade(t); 
          }

          if (snapshot.last_liquidation) handleLiquidation(snapshot.last_liquidation);

          // 3. Process Candles
          if (chartMode === 'candle' && snapshot.last_kline) {
             const k = snapshot.last_kline.k;
             setChartData({
                time: k.t / 1000,
                open: parseFloat(k.o),
                high: parseFloat(k.h),
                low: parseFloat(k.l),
                close: parseFloat(k.c),
             });
          }

        } catch (err) { }
      };
    };

    connect();
    return () => {
      isMounted.current = false;
      clearTimeout(reconnectTimeout);
      ws?.close();
    };
  }, [chartMode, timeframe]);

  // #
  // # HANDLERS
  // #

  const handleTrade = (t: AggTrade) => {
    // Basic dedupe: if price and qty match the LAST trade we added, skip
    // This prevents the 200ms snapshot from spamming the feed with the same inactive trade.
    if (tradesRef.current.length > 0) {
        const last = tradesRef.current[0];
        if (last.price === parseFloat(t.p).toFixed(2) && last.qty === t.q) return;
    }

    const newItem: FeedItem = {
      id: Date.now() + Math.random(),
      text: 'Trade',
      side: t.m ? 'sell' : 'buy',
      time: new Date().toLocaleTimeString(),
      price: parseFloat(t.p).toFixed(2),
      qty: t.q
    };
    const newTrades = [newItem, ...tradesRef.current].slice(0, 50);
    tradesRef.current = newTrades;
    setTrades(newTrades);
  };

  const handleLiquidation = (l: ForceOrder) => {
    const newItem: FeedItem = {
      id: Date.now() + Math.random(),
      text: 'Liquidation',
      side: l.o.S === 'BUY' ? 'buy' : 'sell',
      time: new Date().toLocaleTimeString(),
      price: parseFloat(l.o.p).toFixed(2),
      qty: l.o.q
    };
    const newLiqs = [newItem, ...liqRef.current].slice(0, 30);
    liqRef.current = newLiqs;
    setLiquidations(newLiqs);
  };

  const formatPrice = (p: string) => parseFloat(p).toLocaleString('en-US', { minimumFractionDigits: 2 });

  return (
    <div className="h-screen bg-[#161a25] text-[#b7bdc6] font-mono flex flex-col overflow-hidden">
      
      {/* HEADER BAR */}
      <header className="h-14 bg-[#1e2329] border-b border-[#2b3139] flex items-center px-4 justify-between shrink-0">
        <div className="flex items-center gap-6">
          <div className="flex items-center gap-2">
            <h1 className="text-lg font-bold text-[#eaecef]">BTCUSDT</h1>
            <span className="text-xs bg-[#2b3139] px-1 rounded text-[#848e9c]">Perp</span>
          </div>
          
          <div className="flex flex-col">
            <span className={`text-base font-medium ${ticker && parseFloat(ticker.b) > parseFloat(ticker.a) ? 'text-[#0ecb81]' : 'text-[#f6465d]'}`}>
               {ticker ? formatPrice(ticker.b) : '0.00'}
            </span>
            <span className="text-[10px] text-[#848e9c]">$ {ticker ? formatPrice(ticker.b) : '0.00'}</span>
          </div>

          <div className="flex flex-col">
            <span className="text-[10px] text-[#848e9c]">Mark</span>
            <span className="text-xs text-[#eaecef]">{markPrice ? formatPrice(markPrice.p) : '---'}</span>
          </div>

          <div className="flex flex-col">
            <span className="text-[10px] text-[#848e9c]">Funding</span>
            <span className="text-xs text-[#e6a323]">{markPrice ? (parseFloat(markPrice.r) * 100).toFixed(4) : '---'}%</span>
          </div>
        </div>
        
        <div className={`flex items-center gap-2 px-2 py-1 rounded text-[10px] ${connected ? 'text-[#0ecb81]' : 'text-[#f6465d]'}`}>
          {connected ? <Wifi size={12} /> : <RefreshCw size={12} className="animate-spin" />}
          {connected ? 'Stable' : 'Connecting'}
        </div>
      </header>

      {/* MAIN GRID */}
      <div className="flex flex-1 overflow-hidden">
        
        {/* LEFT: CHART AREA */}
        <div className="flex-1 flex flex-col bg-[#161a25] relative border-r border-[#2b3139]">
          <div className="h-10 border-b border-[#2b3139] flex items-center px-4 gap-4">
             <span className="text-sm font-bold text-[#eaecef]">Time</span>
             {['1m', '5m', '15m', '1h', '1d'].map(tf => (
               <button
                 key={tf}
                 onClick={() => { setChartMode('candle'); setTimeframe(tf); }}
                 className={`text-xs font-medium hover:text-[#f0b90b] ${timeframe === tf && chartMode === 'candle' ? 'text-[#f0b90b]' : 'text-[#848e9c]'}`}
               >
                 {tf}
               </button>
             ))}
             <div className="w-[1px] h-4 bg-[#2b3139] mx-2" />
             <button 
                onClick={() => setChartMode('tick')}
                className={`text-xs font-medium hover:text-[#f0b90b] ${chartMode === 'tick' ? 'text-[#f0b90b]' : 'text-[#848e9c]'}`}
             >
                Tick
             </button>
          </div>

          <div className="flex-1 relative">
             <div className="absolute inset-0">
               <PriceChart 
                 data={chartData} 
                 mode={chartMode} 
                 symbol="BTCUSDT" 
                 timeframe={timeframe} 
               />
             </div>
          </div>
        </div>

        {/* RIGHT: SIDEBAR */}
        <div className="w-[320px] flex flex-col shrink-0 bg-[#161a25]">
          <div className="flex-1 flex flex-col min-h-0 border-b border-[#2b3139]">
             <div className="h-8 flex items-center px-3 border-b border-[#2b3139]">
                <span className="text-xs font-bold text-[#eaecef] flex items-center gap-1">
                   <Layers size={12} /> Order Book
                </span>
             </div>
             
             <div className="flex justify-between px-3 py-1 text-[10px] text-[#848e9c]">
                <span>Price(USDT)</span>
                <span>Size(BTC)</span>
             </div>

             <div className="flex-1 overflow-hidden flex flex-col-reverse justify-end pb-1 gap-[1px]">
                <div className="flex justify-between px-3 text-xs relative hover:bg-[#2b3139] cursor-pointer group">
                   <div className="absolute right-0 top-0 h-full bg-[#f6465d] opacity-10 w-[40%]" />
                   <span className="text-[#f6465d] z-10">{ticker?.a}</span>
                   <span className="text-[#eaecef] z-10">{ticker?.A}</span>
                </div>
             </div>

             <div className="h-8 flex items-center justify-center border-y border-[#2b3139] bg-[#1e2329]">
                <span className={`text-lg font-bold ${ticker && parseFloat(ticker.b) > parseFloat(ticker.a) ? 'text-[#0ecb81]' : 'text-[#f6465d]'}`}>
                   {ticker ? formatPrice(ticker.b) : '---'}
                </span>
                <ArrowUp size={12} className="ml-1 text-[#0ecb81]" />
             </div>

             <div className="flex-1 overflow-hidden pt-1 gap-[1px]">
                 <div className="flex justify-between px-3 text-xs relative hover:bg-[#2b3139] cursor-pointer group">
                   <div className="absolute right-0 top-0 h-full bg-[#0ecb81] opacity-10 w-[60%]" />
                   <span className="text-[#0ecb81] z-10">{ticker?.b}</span>
                   <span className="text-[#eaecef] z-10">{ticker?.B}</span>
                </div>
             </div>
          </div>

          <div className="h-[250px] flex flex-col min-h-0">
             <div className="h-8 flex items-center px-3 border-b border-[#2b3139] justify-between">
                <span className="text-xs font-bold text-[#eaecef] flex items-center gap-1">
                   <History size={12} /> Market Trades
                </span>
             </div>
             <div className="flex justify-between px-3 py-1 text-[10px] text-[#848e9c]">
                <span>Price(USDT)</span>
                <span>Amount(BTC)</span>
                <span>Time</span>
             </div>
             
             <div className="flex-1 overflow-y-auto custom-scrollbar">
                {trades.map((t) => (
                   <div key={t.id} className="flex justify-between px-3 py-[2px] text-[11px] hover:bg-[#2b3139]">
                      <span className={t.side === 'buy' ? 'text-[#0ecb81]' : 'text-[#f6465d]'}>
                         {t.price}
                      </span>
                      <span className="text-[#eaecef]">{t.qty}</span>
                      <span className="text-[#848e9c] text-[10px]">{t.time.split(' ')[0]}</span>
                   </div>
                ))}
             </div>
          </div>

        </div>
      </div>
    </div>
  );
}