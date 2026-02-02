// @file: src/PriceChart.tsx
// @description: Multi-mode Chart (Ticks & Candles) using Lightweight Charts v5.
// This component provides high-performance rendering for financial time-series data.

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
  data: number | CandleData | null; // Can be a single price or a full candle
  mode: ChartMode;
  symbol: string;
  timeframe?: string; // Optional: To display the active timeframe in header
}

// #
// # COMPONENT DEFINITION
// #

export const PriceChart: React.FC<PriceChartProps> = ({ data, mode, symbol, timeframe }) => {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  
  // Ref to hold the active series instance
  const seriesRef = useRef<ISeriesApi<"Area" | "Candlestick"> | null>(null);

  // #1. Initialize & Manage Chart Lifecycle
  // We recreate the chart on mode/timeframe/symbol changes to ensure 
  // the coordinate systems and data types remain consistent.
  useEffect(() => {
    if (!chartContainerRef.current) return;

    // Create Chart Instance
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
        shiftVisibleRangeOnNewBar: true,
      },
    });

    chartRef.current = chart;

    // Create Initial Series based on current Mode
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

    // Handle Window Resizing
    const handleResize = () => {
      if (chartContainerRef.current && chartRef.current) {
        chartRef.current.applyOptions({ width: chartContainerRef.current.clientWidth });
      }
    };
    window.addEventListener('resize', handleResize);

    // Component Cleanup
    return () => {
      window.removeEventListener('resize', handleResize);
      if (chartRef.current) {
        chartRef.current.remove();
        chartRef.current = null;
        seriesRef.current = null;
      }
    };
  }, [mode, timeframe, symbol]);

  // #2. Real-time Data Update Logic
  useEffect(() => {
    if (!data || !seriesRef.current) return;

    try {
      if (mode === 'tick' && typeof data === 'number') {
        const s = seriesRef.current as ISeriesApi<"Area">;
        s.update({
          time: (Date.now() / 1000) as Time,
          value: data,
        });
      } else if (mode === 'candle' && typeof data === 'object') {
        const s = seriesRef.current as ISeriesApi<"Candlestick">;
        const c = data as CandleData;
        
        // Ensure data is valid before updating the chart
        if (c.time && !isNaN(c.open)) {
          s.update({
            time: c.time as Time,
            open: c.open,
            high: c.high,
            low: c.low,
            close: c.close,
          } as CandlestickData<Time>);
        }
      }
    } catch (err) {
      // Catching timestamp or sequence errors from the data feed
      console.warn("Chart series update failed:", err);
    }
  }, [data, mode]);

  return (
    <div className="w-full h-full bg-[#161a25] relative group overflow-hidden">
      {/* Chart Header Information Overlay */}
      <div className="absolute top-2 left-2 z-10 pointer-events-none">
        <div className="text-[10px] font-bold text-[#848e9c] uppercase tracking-wider bg-[#161a25]/90 px-2 py-1 rounded border border-[#2b3139] shadow-lg">
          {symbol} {mode === 'tick' ? 'Ticks' : `Candles (${timeframe || '1m'})`}
        </div>
      </div>
      <div ref={chartContainerRef} className="w-full h-full" />
    </div>
  );
};