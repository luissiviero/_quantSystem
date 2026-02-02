// @file: src/PriceChart.tsx
// @description: Multi-mode Chart (Ticks & Candles) using Lightweight Charts v5.
// Refactored to prevent 'removeSeries' crash by recreating chart on mode change.

import React, { useEffect, useRef } from 'react';
import { 
  createChart, 
  ColorType, 
  IChartApi, 
  ISeriesApi, 
  Time, 
  AreaSeries, 
  CandlestickSeries,
  CandlestickData
} from 'lightweight-charts';

// #
// # TYPE DEFINITIONS
// #

export type ChartMode = 'tick' | 'candle';

export interface CandleData {
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
}

interface PriceChartProps {
  data: number | CandleData | null;
  mode: ChartMode;
  symbol: string;
  timeframe?: string;
}

// #
// # COMPONENT DEFINITION
// #

export const PriceChart: React.FC<PriceChartProps> = ({ data, mode, symbol, timeframe }) => {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  
  // We use refs to hold instances, but we won't try to reuse the chart across modes
  // to avoid the 'removeSeries' crash. We just recreate it.
  const chartRef = useRef<IChartApi | null>(null);
  const seriesRef = useRef<ISeriesApi<"Area" | "Candlestick"> | null>(null);

  // #1. Initialize & Manage Chart Lifecycle
  useEffect(() => {
    if (!chartContainerRef.current) return;

    // 1. Create Chart Instance
    const chart = createChart(chartContainerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: 'transparent' },
        textColor: '#94a3b8',
      },
      grid: {
        vertLines: { color: '#1e293b' },
        horzLines: { color: '#1e293b' },
      },
      width: chartContainerRef.current.clientWidth,
      height: 300,
      timeScale: {
        timeVisible: true,
        secondsVisible: true,
      },
    });

    chartRef.current = chart;

    // 2. Create Series based on current Mode immediately
    let series: ISeriesApi<"Area" | "Candlestick">;

    if (mode === 'tick') {
      series = chart.addSeries(AreaSeries, {
        lineColor: '#10b981',
        topColor: 'rgba(16, 185, 129, 0.4)',
        bottomColor: 'rgba(16, 185, 129, 0.0)',
        lineWidth: 2,
      });
    } else {
      series = chart.addSeries(CandlestickSeries, {
        upColor: '#10b981',
        downColor: '#ef4444',
        borderVisible: false,
        wickUpColor: '#10b981',
        wickDownColor: '#ef4444',
      });
    }
    
    seriesRef.current = series;

    // 3. Resize Handler
    const handleResize = () => {
      if (chartContainerRef.current) {
        chart.applyOptions({ width: chartContainerRef.current.clientWidth });
      }
    };
    window.addEventListener('resize', handleResize);

    // 4. Cleanup
    return () => {
      window.removeEventListener('resize', handleResize);
      chart.remove(); // Destroys chart and all series safely
      chartRef.current = null;
      seriesRef.current = null;
    };
  }, [mode]); // Re-run everything if Mode changes

  // #2. Data Update Logic
  // This runs whenever 'data' changes, using the EXISTING chart instance
  useEffect(() => {
    if (!data || !seriesRef.current) return;

    try {
      if (mode === 'tick' && typeof data === 'number') {
        // Force type cast because we know it matches the mode
        const s = seriesRef.current as ISeriesApi<"Area">;
        s.update({
          time: (Date.now() / 1000) as Time,
          value: data,
        });
      } else if (mode === 'candle' && typeof data === 'object') {
        const s = seriesRef.current as ISeriesApi<"Candlestick">;
        const c = data as CandleData;
        s.update({
          time: c.time as Time,
          open: c.open,
          high: c.high,
          low: c.low,
          close: c.close,
        } as CandlestickData<Time>);
      }
    } catch (err) {
      console.warn("Chart update failed:", err);
    }
  }, [data, mode]); // Depend on data and mode

  return (
    <div className="w-full bg-[#161a25] border border-[#2b3139] rounded-xl p-0 relative group">
      {/* Header Overlay */}
      <div className="absolute top-2 left-2 z-10 text-xs font-bold text-[#848e9c] uppercase tracking-wider bg-[#161a25]/80 px-2 py-1 rounded backdrop-blur-sm">
        {symbol} {mode === 'tick' ? 'Ticks' : `Candles ${timeframe ? `(${timeframe})` : ''}`}
      </div>
      <div ref={chartContainerRef} className="w-full h-full" />
    </div>
  );
};