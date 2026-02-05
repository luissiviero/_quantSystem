/**
 * @file: PriceChart.tsx
 * @description: Renders a financial candlestick chart using lightweight-charts.
 * Includes a timeframe switcher overlay to control data granularity.
 * @tags: #component #chart #visualization #lightweight-charts
 */

import { useEffect, useRef, useCallback } from 'react';
import { createChart, ColorType, IChartApi, ISeriesApi, CandlestickData, Time } from 'lightweight-charts';
import { useCandleStore } from '../store/useCandleStore';
import { useConnectionStore } from '../store/useConnectionStore';
import { Candle } from '../models/types';

//
// CONSTANTS
//

const TIMEFRAMES: string[] = ['1m', '5m', '15m', '1h', '4h', '1d'];

//
// COMPONENT
//

export default function PriceChart() {
  //
  // STATE & REFS
  //

  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartApiRef = useRef<IChartApi | null>(null);
  const seriesApiRef = useRef<ISeriesApi<"Candlestick"> | null>(null);
  
  // Track the timestamp of the last successfully processed candle
  const lastCandleTimeRef = useRef<number>(0);

  // #1. Get data from CandleStore
  const { candles, activeTimeframe } = useCandleStore();
  
  // #2. Get Action from ConnectionStore (Controller)
  const { switchTimeframe } = useConnectionStore();

  //
  // HELPER FUNCTIONS
  //

  const handleResize = useCallback(() => {
    if (chartApiRef.current && chartContainerRef.current) {
      chartApiRef.current.applyOptions({ 
        width: chartContainerRef.current.clientWidth,
        height: chartContainerRef.current.clientHeight 
      });
    }
  }, []);

  //
  // INITIALIZATION EFFECT
  //

  useEffect(() => {
    if (!chartContainerRef.current) return;

    // #1. Create Chart Instance
    const chart = createChart(chartContainerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: '#111827' }, // Gray-900
        textColor: '#9CA3AF',
      },
      grid: {
        vertLines: { color: '#374151' },
        horzLines: { color: '#374151' },
      },
      width: chartContainerRef.current.clientWidth,
      height: chartContainerRef.current.clientHeight,
      timeScale: {
        timeVisible: true,
        secondsVisible: true,
        borderColor: '#374151',
      },
      rightPriceScale: {
        borderColor: '#374151',
      }
    });

    // #2. Add Candlestick Series
    const newSeries = chart.addCandlestickSeries({
      upColor: '#10B981',    // Emerald-500
      downColor: '#EF4444',  // Red-500
      borderVisible: false,
      wickUpColor: '#10B981',
      wickDownColor: '#EF4444',
    });

    chartApiRef.current = chart;
    seriesApiRef.current = newSeries;

    // #3. Setup Resize Observer
    window.addEventListener('resize', handleResize);

    // #4. Cleanup
    return () => {
      window.removeEventListener('resize', handleResize);
      chart.remove();
      chartApiRef.current = null;
      seriesApiRef.current = null;
    };
  }, [handleResize]);

  //
  // DATA SYNC EFFECT
  //

  useEffect(() => {
    if (!seriesApiRef.current) return;

    // #1. Reset Logic
    if (candles.length === 0) {
        seriesApiRef.current.setData([]);
        lastCandleTimeRef.current = 0;
        return;
    }

    // #2. Convert store candles to Chart format
    const formattedData: CandlestickData[] = candles.map((c: Candle) => ({
        time: (c.start_time / 1000) as Time, 
        open: c.open,
        high: c.high,
        low: c.low,
        close: c.close,
    }));

    // #3. SAFETY DEDUPLICATION
    const uniqueMap = new Map();
    for (const item of formattedData) {
        uniqueMap.set(item.time, item);
    }
    const uniqueData = Array.from(uniqueMap.values());

    // #4. Sort Strict Ascending
    uniqueData.sort((a, b) => (a.time as number) - (b.time as number));

    if (uniqueData.length === 0) return;

    const newestCandidate = uniqueData[uniqueData.length - 1];
    const newestTime = newestCandidate.time as number;

    // #5. Update Logic
    const isHistoryLoad = lastCandleTimeRef.current === 0 || uniqueData.length > 5;

    if (isHistoryLoad) {
        try {
            seriesApiRef.current.setData(uniqueData);
            lastCandleTimeRef.current = newestTime;
            
            if (chartApiRef.current) {
                chartApiRef.current.timeScale().fitContent();
            }
        } catch (err) {
            console.error("Critical: Failed to set chart history data", err);
        }
    } else {
        if (newestTime >= lastCandleTimeRef.current) {
            try {
                seriesApiRef.current.update(newestCandidate);
                lastCandleTimeRef.current = newestTime;
            } catch (err) {
                console.warn("Chart update rejected by library", err);
            }
        }
    }

  }, [candles]);

  //
  // RENDER
  //

  return (
    <div className="w-full h-full p-1 bg-gray-900 relative group">
        
        {/* TIMEFRAME SWITCHER OVERLAY */}
        <div className="absolute top-3 left-3 z-20 flex gap-1 bg-gray-800/80 backdrop-blur-sm p-1 rounded-md border border-gray-700 shadow-lg opacity-80 group-hover:opacity-100 transition-opacity">
            {TIMEFRAMES.map((tf) => (
                <button
                    key={tf}
                    // Updated to use Controller Action
                    onClick={() => switchTimeframe(tf)}
                    className={`
                        px-2 py-1 text-xs font-mono font-bold rounded cursor-pointer transition-colors
                        ${activeTimeframe === tf 
                            ? 'bg-blue-600 text-white shadow-sm' 
                            : 'text-gray-400 hover:text-white hover:bg-gray-700'}
                    `}
                >
                    {tf.toUpperCase()}
                </button>
            ))}
        </div>

        {/* CHART CONTAINER */}
        <div ref={chartContainerRef} className="w-full h-full" />
    </div>
  );
}