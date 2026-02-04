// @file: PriceChart.tsx
// @description: Candlestick chart with infinite scroll history loading and defensive sorting.
// @author: v5 helper

import React, { useEffect, useRef } from 'react';
import { 
  createChart, 
  ColorType, 
  IChartApi, 
  ISeriesApi, 
  Time,
} from 'lightweight-charts';
import { useMarketStore } from '../store/useMarketStore';
import { Command } from '../models/types';

const PriceChart: React.FC = () => {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const seriesRef = useRef<ISeriesApi<"Candlestick"> | null>(null);
  
  const { candles, latestCandle, isLoadingHistory, setLoadingHistory } = useMarketStore();
  
  // 1. Send History Request Helper
  const requestHistory = (endTime: number) => {
      const event = new CustomEvent('ws-send', { 
          detail: { 
              action: 'fetch_history', 
              channel: 'BTCUSDT', 
              end_time: endTime 
          } 
      });
      window.dispatchEvent(event);
      setLoadingHistory(true);
  };

  // 2. Chart Initialization
  useEffect(() => {
    if (!chartContainerRef.current) return;

    const chart = createChart(chartContainerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: '#1f2937' },
        textColor: '#9ca3af',
      },
      grid: {
        vertLines: { color: '#374151' },
        horzLines: { color: '#374151' },
      },
      width: chartContainerRef.current.clientWidth,
      height: chartContainerRef.current.clientHeight,
      timeScale: {
        timeVisible: true,
        secondsVisible: false,
      },
    });

    const newSeries = chart.addCandlestickSeries({
        upColor: '#22c55e',
        downColor: '#ef4444',
        borderVisible: false,
        wickUpColor: '#22c55e',
        wickDownColor: '#ef4444',
    });

    chartRef.current = chart;
    seriesRef.current = newSeries;

    // #3. INFINITE SCROLL LOGIC
    chart.timeScale().subscribeVisibleLogicalRangeChange((newLogicalRange) => {
        if (!newLogicalRange) return;

        if (newLogicalRange.from < 10 && !useMarketStore.getState().isLoadingHistory) {
            const currentCandles = useMarketStore.getState().candles;
            if (currentCandles.length > 0) {
                const oldestTime = currentCandles[0].start_time;
                console.log("Loading history before:", oldestTime);
                requestHistory(oldestTime);
            }
        }
    });

    const handleResize = () => {
      chart.applyOptions({ 
        width: chartContainerRef.current?.clientWidth || 0,
        height: chartContainerRef.current?.clientHeight || 0 
      });
    };

    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      chart.remove();
    };
  }, []);

  // 3. React to Data Changes (Initial Load & Prepend)
  useEffect(() => {
    if (!seriesRef.current || candles.length === 0) return;

    // Map store candles to Chart format
    const data = candles.map(c => ({
        time: (c.start_time / 1000) as Time,
        open: c.open,
        high: c.high,
        low: c.low,
        close: c.close
    }));

    // FAIL-SAFE: Sort explicitly in the view layer to prevent crashes
    data.sort((a, b) => (a.time as number) - (b.time as number));

    seriesRef.current.setData(data);
  }, [candles.length]); 

  // 4. React to Live Updates (Incremental)
  useEffect(() => {
      if (!seriesRef.current || !latestCandle) return;
      
      seriesRef.current.update({
          time: (latestCandle.start_time / 1000) as Time,
          open: latestCandle.open,
          high: latestCandle.high,
          low: latestCandle.low,
          close: latestCandle.close
      });
  }, [latestCandle]);

  return (
    <div className="bg-gray-800 rounded-lg border border-gray-700 h-full w-full flex flex-col relative">
       {isLoadingHistory && (
           <div className="absolute top-4 left-1/2 transform -translate-x-1/2 bg-blue-600 text-white text-xs px-3 py-1 rounded-full shadow-lg z-10 animate-pulse">
               Loading History...
           </div>
       )}
       <div ref={chartContainerRef} className="flex-1 w-full h-full overflow-hidden" />
    </div>
  );
};

export default PriceChart;