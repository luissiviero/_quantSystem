# @file: stress_test.py
# @description: External load testing client to verify WebSocket throughput with multi-exchange support.
# @author: v5 helper

import asyncio
import websockets
import json
import time
from typing import Dict, Any, Union

#
# CONSTANTS
#

SERVER_URL: str = "ws://127.0.0.1:8080"
TEST_DURATION_SEC: int = 10
SYMBOL: str = "BTCUSDT"
EXCHANGE: str = "BINANCE"       # Options: BINANCE, BYBIT, COINBASE
MARKET_TYPE: str = "SPOT"       # Options: SPOT, LINEAR_FUTURE, INVERSE_FUTURE

#
# LOAD TEST LOGIC
#

async def run_stress_test() -> None:
    print(f"Connecting to {SERVER_URL}...")

    try:
        # #1. Establish Connection
        async with websockets.connect(SERVER_URL) as websocket:
            print("Connected.")

            # #2. Send Subscribe Command (Updated for new Architecture)
            subscribe_cmd: Dict[str, Any] = {
                "action": "subscribe",
                "channel": SYMBOL,
                "exchange": EXCHANGE,
                "market_type": MARKET_TYPE
            }
            
            await websocket.send(json.dumps(subscribe_cmd))
            print(f"Subscribed to {EXCHANGE}:{MARKET_TYPE}:{SYMBOL}. Measuring throughput for {TEST_DURATION_SEC} seconds...")

            # #3. Measurement Loop
            msg_count: int = 0
            start_time: float = time.time()
            
            while True:
                try:
                    # Set a timeout so we can exit the loop eventually
                    # Fix: Explicitly type hint as Union[str, bytes] to satisfy Pylance
                    message: Union[str, bytes] = await asyncio.wait_for(websocket.recv(), timeout=1.0)
                    
                    # Verify it's valid JSON (Lightweight check)
                    data: Dict[str, Any] = json.loads(message)
                    msg_count += 1
                    
                    # Check Duration
                    current_time: float = time.time()
                    if current_time - start_time >= TEST_DURATION_SEC:
                        break

                except asyncio.TimeoutError:
                    # No messages received recently
                    current_time: float = time.time()
                    if current_time - start_time >= TEST_DURATION_SEC:
                        break
                except Exception as e:
                    print(f"Error during receive loop: {e}")
                    break

            # #4. Report Results
            elapsed: float = time.time() - start_time
            if elapsed == 0: elapsed = 0.001  # Prevent division by zero
            mps: float = msg_count / elapsed

            print("\n" + "="*40)
            print("EXTERNAL LOAD TEST RESULTS")
            print("="*40)
            print(f"Target            : {EXCHANGE} {MARKET_TYPE} {SYMBOL}")
            print(f"Messages Received : {msg_count}")
            print(f"Time Elapsed      : {elapsed:.2f}s")
            print(f"Throughput        : {mps:.2f} msgs/sec")
            print("="*40 + "\n")

    except ConnectionRefusedError:
        print(f"\n[Error] Connection Refused by {SERVER_URL}")
        print("Diagnosis: The Rust WebSocket server is likely not running.")
        print("Action: Open a new terminal and run 'cargo run' in the ingestion_engine directory.\n")
    except Exception as e:
        print(f"\n[Fatal Error] {e}\n")

#
# MAIN EXECUTION
#

if __name__ == "__main__":
    try:
        asyncio.run(run_stress_test())
    except KeyboardInterrupt:
        print("Test stopped by user.")