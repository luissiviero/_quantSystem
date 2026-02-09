# @file: ws_monitor.py
# @description: A strictly typed Python client to monitor the ingestion engine with a 1s refresh dashboard.
# @author: V5 Helper

import websocket  # type: ignore
import json
import sys
import time
import threading
from typing import Dict, Any, Optional, List

#
# CONSTANTS
#

SERVER_URL: str = "ws://127.0.0.1:8080"

# Define targets to monitor to demonstrate "All Market Types"
SUBSCRIPTIONS: List[Dict[str, Any]] = [
    # 1. Spot Market (Basic features: Trade, Book, Ticker)
    {
        "exchange": "BINANCE",
        "market_type": "SPOT",
        "channel": "BTCUSDT",
        "config": {
            "raw_trades": True,
            "agg_trades": True,
            "order_book": True,
            "ticker": True,
            "book_ticker": True,
            "kline_intervals": ["1m"]
        }
    },
    # 2. Linear Futures (Derivatives features: MarkPrice, Funding, Liquidation, OI)
    {
        "exchange": "BINANCE",
        "market_type": "LINEAR_FUTURE",
        "channel": "BTCUSDT",
        "config": {
            "raw_trades": True,
            "ticker": True,
            "mark_price": True,
            "liquidation": True,
            "funding_rate": True,
            "open_interest": True,
            "kline_intervals": []
        }
    }
]

# ANSI Escape Codes for Terminal Control
ANSI_UP: str = "\033[F"
ANSI_CLEAR: str = "\033[K"


#
# STATE MANAGEMENT
#

class RowData:
    """
    Data container for a single symbol's display row.
    """
    def __init__(self, uid: str) -> None:
        self.uid: str = uid
        self.last_trade: str = "-"
        self.mark_price: str = "-"
        self.funding: str = "-"
        self.oi: str = "-"
        self.last_liq: str = "-"
        self.ticker_price: str = "-"

class MonitorState:
    """
    Thread-safe container for the latest market data across all subscriptions.
    """
    def __init__(self) -> None:
        self.lock: threading.Lock = threading.Lock()
        self.rows: Dict[str, RowData] = {}
        self.msg_count: int = 0
        
        # Pre-initialize rows for stable rendering
        for sub in SUBSCRIPTIONS:
            uid = f"{sub['exchange']}_{sub['market_type']}_{sub['channel']}".upper()
            self.rows[uid] = RowData(uid)

# Global state instance
state: MonitorState = MonitorState()


#
# DISPLAY LOGIC
#

def render_dashboard(first_run: bool) -> None:
    """
    Prints a dynamic table. Moves cursor up based on row count.
    """
    # 1. Acquire lock to read consistent state
    snapshot: List[RowData] = []
    count: int = 0
    
    with state.lock:
        count = state.msg_count
        # Sort by UID for consistent display order
        for uid in sorted(state.rows.keys()):
            snapshot.append(state.rows[uid])

    # 2. Calculate height to move cursor
    # Header (4 lines) + Rows (len(snapshot))
    lines_to_clear: int = 4 + len(snapshot)

    if not first_run:
        sys.stdout.write((ANSI_UP + ANSI_CLEAR) * lines_to_clear)

    # 3. Print Header
    print("-" * 140)
    print(f"| MONITOR (1s Refresh) | MSG COUNT: {count:<10} | URL: {SERVER_URL:<67} |")
    print("-" * 140)
    # Column Headers
    print(f"| {'ID':<35} | {'TRADE PRICE':<15} | {'MARK PRICE':<15} | {'FUNDING RATE':<15} | {'OPEN INTEREST':<15} | {'LATEST LIQUIDATION':<30} |")
    print("-" * 140)

    # 4. Print Rows
    for row in snapshot:
        # Use ticker price if trade price is empty/dash, else trade
        display_price = row.last_trade if row.last_trade != "-" else row.ticker_price
        
        print(f"| {row.uid:<35} | {display_price:<15} | {row.mark_price:<15} | {row.funding:<15} | {row.oi:<15} | {row.last_liq:<30} |")
    
    sys.stdout.flush()


#
# WEBSOCKET EVENT HANDLERS
#

def on_message(ws: websocket.WebSocketApp, message: str) -> None:
    """
    Parses incoming messages and routes them to the correct RowData.
    """
    try:
        parsed: Dict[str, Any] = json.loads(message)
        
        with state.lock:
            state.msg_count += 1
            
            if "type" in parsed and "data" in parsed:
                msg_type: str = str(parsed["type"])
                data: Dict[str, Any] = parsed["data"]
                
                # The Engine sends the unique_id as "symbol" in the data payload
                uid: str = str(data.get("symbol", "")).upper()
                
                if uid in state.rows:
                    row = state.rows[uid]
                    
                    if msg_type == "Trade":
                        price: float = float(data.get("price", 0.0))
                        row.last_trade = f"{price:.2f}"
                        
                    elif msg_type == "Ticker":
                        price = float(data.get("last_price", 0.0))
                        row.ticker_price = f"({price:.2f})"
                        
                    elif msg_type == "MarkPrice":
                        mp: float = float(data.get("mark_price", 0.0))
                        row.mark_price = f"{mp:.2f}"
                        
                    elif msg_type == "FundingRate":
                        rate: float = float(data.get("rate", 0.0))
                        # Funding rate is usually small, show as percentage
                        row.funding = f"{rate * 100:.4f}%"
                        
                    elif msg_type == "OpenInterest":
                        oi: float = float(data.get("open_interest", 0.0))
                        row.oi = f"{oi:.2f}"
                        
                    elif msg_type == "Liquidation":
                        side: str = str(data.get("side", "?"))
                        qty: float = float(data.get("quantity", 0.0))
                        lprice: float = float(data.get("price", 0.0))
                        row.last_liq = f"{side} {qty:.3f} @ {lprice:.2f}"

    except Exception:
        pass


def on_error(ws: websocket.WebSocketApp, error: Any) -> None:
    sys.stdout.write(f"\n[ERROR] {error}\n")


def on_close(ws: websocket.WebSocketApp, close_status_code: Optional[int], close_msg: Optional[str]) -> None:
    sys.stdout.write("\n[INFO] Connection closed.\n")


def on_open(ws: websocket.WebSocketApp) -> None:
    """
    Sends all subscriptions on connect.
    """
    for sub in SUBSCRIPTIONS:
        payload: Dict[str, Any] = {
            "action": "subscribe",
            "channel": sub["channel"],
            "exchange": sub["exchange"],
            "market_type": sub["market_type"],
            "config": sub["config"]
        }
        ws.send(json.dumps(payload))
        # Small delay to prevent server buffer overload during handshake
        time.sleep(0.1)


#
# THREAD RUNNER
#

def run_ws_client() -> None:
    """
    Wrapper to run the blocking WebSocket app.
    """
    # websocket.enableTrace(True)
    ws_app: websocket.WebSocketApp = websocket.WebSocketApp(
        SERVER_URL,
        on_open=on_open,
        on_message=on_message,
        on_error=on_error,
        on_close=on_close
    )
    ws_app.run_forever()


#
# MAIN EXECUTION
#

if __name__ == "__main__":
    print(f"Starting Monitor for {SERVER_URL}...")
    
    t: threading.Thread = threading.Thread(target=run_ws_client)
    t.daemon = True
    t.start()

    time.sleep(1)

    try:
        is_first: bool = True
        while True:
            render_dashboard(is_first)
            is_first = False
            time.sleep(1)
            
    except KeyboardInterrupt:
        print("\n[INFO] User interrupted.")