// dashboard/src/components/PriceChart.tsx
import { useEffect, useRef } from 'react';
import { useMarketStore } from '../store/useMarketStore';

declare global {
    interface Window {
        LightweightCharts: any;
    }
}

const PriceChart = () => {
    const chartContainerRef = useRef<HTMLDivElement>(null);
    const activeSymbol = useMarketStore((state) => state.activeSymbol);

    useEffect(() => {
        if (!chartContainerRef.current) return;
        
        if (!window.LightweightCharts) {
            console.error("LightweightCharts library not found.");
            return;
        }

        const chart = window.LightweightCharts.createChart(chartContainerRef.current, {
            layout: { 
                textColor: '#d1d4dc', 
                background: { type: 'solid', color: '#131722' } 
            },
            grid: {
                vertLines: { color: '#2B2B43' },
                horzLines: { color: '#2B2B43' },
            },
            width: chartContainerRef.current.clientWidth,
            height: 500,
        });

        // Note: Using v5.0 syntax: .addSeries()
        const lineSeries = chart.addSeries(window.LightweightCharts.LineSeries, { 
            color: '#2962FF',
            lineWidth: 2,
        });

        const handleResize = () => {
            if (chartContainerRef.current) {
                chart.applyOptions({ 
                    width: chartContainerRef.current.clientWidth 
                });
            }
        };
        window.addEventListener('resize', handleResize);

        const unsubscribe = useMarketStore.subscribe(
            (state) => state.lastTrade,
            (trade) => {
                if (trade && trade.symbol === activeSymbol) {
                    // FIX: Use Math.floor to ensure integer seconds
                    const seconds = Math.floor(trade.timestamp / 1000);
                    
                    // DEBUG: Uncomment if chart is still empty
                    // console.log("Updating Chart:", seconds, trade.price);
                    
                    lineSeries.update({
                        time: seconds, 
                        value: trade.price
                    });
                }
            }
        );

        return () => {
            window.removeEventListener('resize', handleResize);
            unsubscribe();
            chart.remove();
        };
    }, [activeSymbol]);

    return (
        <div className="w-full h-full p-4 bg-gray-900 rounded-lg shadow-lg">
            <h2 className="text-xl font-bold text-white mb-4">
                {activeSymbol} Live Price
            </h2>
            <div 
                ref={chartContainerRef} 
                className="w-full h-[500px] border border-gray-700 rounded"
            />
        </div>
    );
};

export default PriceChart;