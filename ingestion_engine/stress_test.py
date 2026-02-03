# @file: stress_test.py
# @description: External load testing client to verify WebSocket throughput.
# @author: v5 helper

import asyncio
import websockets
import json
import time

#
# CONSTANTS
#

SERVER_URL: str = "ws://127.0.0.1:8080"
TEST_DURATION_SEC: int = 10
SYMBOL: str = "BTCUSDT"

#
# LOAD TEST LOGIC
#

async def run_stress_test() -> None:
    print(f"Connecting to {SERVER_URL}...")

    # #1. Establish Connection
    async with websockets.connect(SERVER_URL) as websocket:
        print("Connected.")

        # #2. Send Subscribe Command
        subscribe_cmd: dict = {
            "action": "subscribe",
            "channel": SYMBOL
        }
        await websocket.send(json.dumps(subscribe_cmd))
        print(f"Subscribed to {SYMBOL}. Measuring throughput for {TEST_DURATION_SEC} seconds...")

        # #3. Measurement Loop
        msg_count: int = 0
        start_time: float = time.time()
        
        while True:
            try:
                # Set a timeout so we can exit the loop eventually
                message: str = await asyncio.wait_for(websocket.recv(), timeout=1.0)
                
                # Verify it's valid JSON (Lightweight check)
                data: dict = json.loads(message)
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
                print(f"Error: {e}")
                break

        # #4. Report Results
        elapsed: float = time.time() - start_time
        mps: float = msg_count / elapsed

        print("\n" + "="*40)
        print("EXTERNAL LOAD TEST RESULTS")
        print("="*40)
        print(f"Messages Received : {msg_count}")
        print(f"Time Elapsed      : {elapsed:.2f}s")
        print(f"Throughput        : {mps:.2f} msgs/sec")
        print("="*40 + "\n")

#
# MAIN EXECUTION
#

if __name__ == "__main__":
    try:
        asyncio.run(run_stress_test())
    except KeyboardInterrupt:
        print("Test stopped by user.")