// @file: PriceChart.tsx
// @description: Wrapper for TradingView's lightweight-charts with proper data buffering.
// @author: v5 helper

import React, { useEffect, useRef } from 'react';
import { createChart, ColorType, IChartApi, ISeriesApi, LineData, Time } from 'lightweight-charts';
import { useMarketStore } from '../store/useMarketStore';
import { Trade } from '../models/types';

//
// COMPONENT LOGIC
//

const PriceChart: React.FC = () => {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const seriesRef = useRef<ISeriesApi<"Area"> | null>(null);
  
  const { recentTrades } = useMarketStore();

  // 1. Initialize Chart
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
        secondsVisible: true,
        borderColor: '#4b5563',
      },
      rightPriceScale: {
        borderColor: '#4b5563',
        scaleMargins: {
            top: 0.1,
            bottom: 0.1,
        }
      },
    });

    const newSeries = chart.addAreaSeries({
      lineColor: '#2563eb',
      topColor: 'rgba(37, 99, 235, 0.4)',
      bottomColor: 'rgba(37, 99, 235, 0)',
      lineWidth: 2,
    });

    chartRef.current = chart;
    seriesRef.current = newSeries;

    const handleResize = () => {
      if (chartContainerRef.current) {
        chart.applyOptions({ 
            width: chartContainerRef.current.clientWidth,
            height: chartContainerRef.current.clientHeight 
        });
      }
    };

    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      chart.remove();
    };
  }, []);

  // 2. Update Data
  useEffect(() => {
    if (!seriesRef.current || recentTrades.length === 0) return;

    // 2a. Transform Data
    // Store is Newest -> Oldest. Chart needs Oldest -> Newest.
    const sortedData = [...recentTrades].reverse().map((t: Trade) => ({
        // Use Seconds for TimeScale
        time: (Math.floor(t.time / 1000)) as Time, 
        value: t.price,
    }));

    // 2b. Deduplicate and take Closing Price
    // Logic: Since sortedData is Oldest->Newest, setting the map key overwrites previous values.
    // This effectively selects the *last* price that occurred in a specific second (Closing Price).
    const uniqueMap = new Map<Time, LineData>();
    
    for (const item of sortedData) {
        uniqueMap.set(item.time as Time, item as LineData);
    }
    
    const uniqueData = Array.from(uniqueMap.values());

    // 2c. Set Data
    seriesRef.current.setData(uniqueData);
    
  }, [recentTrades]);

  return (
    <div className="bg-gray-800 rounded-lg border border-gray-700 h-full w-full p-2 flex flex-col">
      <div className="flex justify-between items-center px-2 mb-2">
         <h2 className="text-sm font-bold text-gray-400">BTC/USDT - Realtime</h2>
         <span className="text-xs text-gray-500">Live Feed</span>
      </div>
      <div ref={chartContainerRef} className="flex-1 w-full h-full overflow-hidden rounded" />
    </div>
  );
};

export default PriceChart;