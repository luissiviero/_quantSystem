// dashboard/src/components/RecentTrades.tsx
import { useMarketStore } from '../store/useMarketStore';

const RecentTrades = () => {
    const trades = useMarketStore((state) => state.tradeHistory);

    return (
        <div className="flex flex-col h-[500px] bg-gray-900 rounded-lg border border-gray-700 overflow-hidden font-mono text-xs">
            <div className="p-2 border-b border-gray-800 font-bold text-center text-gray-400">
                Recent Trades
            </div>
            
            <div className="flex-1 overflow-y-auto">
                <table className="w-full text-left border-collapse">
                    <thead className="sticky top-0 bg-gray-900 text-gray-500 text-[10px] uppercase">
                        <tr>
                            <th className="px-2 py-1">Price</th>
                            <th className="px-2 py-1 text-right">Qty</th>
                            <th className="px-2 py-1 text-right">Time</th>
                        </tr>
                    </thead>
                    <tbody>
                        {trades.map((trade, i) => (
                            <tr key={`${trade.timestamp}-${i}`} className="hover:bg-gray-800 border-b border-gray-800/50 last:border-0">
                                <td className="px-2 py-1 text-blue-300">
                                    {trade.price.toFixed(2)}
                                </td>
                                <td className="px-2 py-1 text-right text-gray-400">
                                    {trade.quantity.toFixed(5)}
                                </td>
                                <td className="px-2 py-1 text-right text-gray-600">
                                    {new Date(trade.timestamp).toLocaleTimeString([], { 
                                        hour: '2-digit', 
                                        minute: '2-digit', 
                                        second: '2-digit' 
                                    })}
                                </td>
                            </tr>
                        ))}
                    </tbody>
                </table>
                {trades.length === 0 && (
                    <div className="text-center text-gray-600 mt-10">Waiting for trades...</div>
                )}
            </div>
        </div>
    );
};

export default RecentTrades;