// dashboard/src/components/OrderBook.tsx
import { useMarketStore } from '../store/useMarketStore';

const OrderBook = () => {
    // Select only what we need to minimize re-renders
    const orderBook = useMarketStore((state) => state.orderBook);

    if (!orderBook) {
        return (
            <div className="h-full flex items-center justify-center text-gray-500 bg-gray-900 rounded-lg border border-gray-700">
                Loading Book...
            </div>
        );
    }

    // Take top 10 asks (lowest price) and reverse them so highest is top visually
    const asks = orderBook.asks.slice(0, 15).reverse();
    const bids = orderBook.bids.slice(0, 15);

    return (
        <div className="flex flex-col h-[500px] bg-gray-900 rounded-lg border border-gray-700 overflow-hidden font-mono text-xs">
            <div className="p-2 border-b border-gray-800 font-bold text-center text-gray-400">
                Order Book
            </div>
            
            <div className="flex-1 overflow-hidden flex flex-col">
                {/* Asks (Sells) - Red */}
                <div className="flex-1 flex flex-col justify-end pb-1">
                    {asks.map(([price, qty], i) => (
                        <div key={i} className="flex justify-between px-2 py-0.5 hover:bg-gray-800 cursor-pointer">
                            <span className="text-red-400">{price.toFixed(2)}</span>
                            <span className="text-gray-500">{qty.toFixed(4)}</span>
                        </div>
                    ))}
                </div>

                {/* Spread / Mid Market could go here */}
                <div className="border-t border-b border-gray-700 h-1 my-1 bg-gray-800"></div>

                {/* Bids (Buys) - Green */}
                <div className="flex-1 pt-1">
                    {bids.map(([price, qty], i) => (
                        <div key={i} className="flex justify-between px-2 py-0.5 hover:bg-gray-800 cursor-pointer">
                            <span className="text-green-400">{price.toFixed(2)}</span>
                            <span className="text-gray-500">{qty.toFixed(4)}</span>
                        </div>
                    ))}
                </div>
            </div>
        </div>
    );
};

export default OrderBook;