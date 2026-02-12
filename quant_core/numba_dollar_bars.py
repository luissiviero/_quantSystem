# @file: test_dollar_bars.py
# @description: Standalone script to compile Dollar Bars with persistent state across days.
# @author: v5 helper

import os
import glob
import sys
import time
import pandas as pd
import numpy as np
from numba import njit
from pathlib import Path
from datetime import datetime
from typing import List, Dict, Tuple, Any

#
# CONSTANTS & CONFIGURATION
#

class TestConfig:
    # Input Paths
    BASE_PATH: Path = Path(r"D:\v5Protocol\binance\BTCUSDC\futures")
    KLINES_PATH: Path = BASE_PATH / "klines" / "1d"
    TRADES_PATH: Path = BASE_PATH / "aggtrades"
    
    # Output Path
    OUTPUT_ROOT: Path = Path(r"D:\dollar_bar_test\numba")
    
    # Algorithm Settings
    DOLLAR_BAR_TARGETS: List[int] = [10000, 4000, 2000, 1000, 500, 200]
    DOLLAR_BAR_WINDOW: int = 30
    
    # File Patterns
    # Expected: "BTCUSDC_futures_aggTrades_{date}.parquet"
    SYMBOL: str = "BTCUSDC"
    MARKET_TYPE: str = "futures"


#
# LOGIC BLOCK 1: PRE-CALCULATION (ADV)
#

def get_rolling_adv(window: int) -> pd.Series:
    """
    Calculates rolling Average Daily Volume (ADV) from 1D Klines.
    """
    print(f"   [ADV] Calculating Rolling {window}-Day ADV...")
    
    # #1. Find all 1D kline files
    search_pattern: str = str(TestConfig.KLINES_PATH / "*.parquet")
    files: List[str] = glob.glob(search_pattern)
    
    if not files:
        print(f"   [WARN] No 1D kline files found in {TestConfig.KLINES_PATH}")
        return pd.Series(dtype=float)

    # #2. Load and merge files
    dfs: List[pd.DataFrame] = []
    for f in files:
        try:
            df: pd.DataFrame = pd.read_parquet(f)
            # Normalize columns to lowercase for safety
            cols: Dict[str, str] = {c.lower(): c for c in df.columns}
            
            if 'quotevolume' in cols and 'opentime' in cols:
                df_sub: pd.DataFrame = df[[cols['opentime'], cols['quotevolume']]].copy()
                df_sub.columns = ['OpenTime', 'QuoteVolume']
                dfs.append(df_sub)
        except Exception as e:
            print(f"   [WARN] Skipping corrupt file {f}: {e}")

    if not dfs:
        return pd.Series(dtype=float)

    full_df: pd.DataFrame = pd.concat(dfs).sort_values('OpenTime')
    
    # #3. Process Time Index
    full_df['Date'] = pd.to_datetime(full_df['OpenTime'], unit='ms')
    full_df.set_index('Date', inplace=True)
    
    # #4. Compute Rolling Stats
    # Resample ensures we have rows for missing days (filled with 0 or NaN)
    daily_vol: pd.Series = full_df['QuoteVolume'].resample('D').sum()
    rolling_adv: pd.Series = daily_vol.rolling(window=window).mean()
    
    # Shift by 1 so Day T uses ADV from T-30 to T-1
    prior_adv: pd.Series = rolling_adv.shift(1).dropna()
    
    print(f"   [ADV] Ready. Coverage: {len(prior_adv)} days.")
    return prior_adv


#
# LOGIC BLOCK 2: NUMBA GENERATOR
#

@njit
def _process_day_chunk(
    timestamps: np.ndarray, 
    prices: np.ndarray, 
    quantities: np.ndarray, 
    threshold: float, 
    state: np.ndarray
) -> Tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray, np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
    """
    Processes ticks for a single day, maintaining state from previous days.
    state structure: [cum_dollar, cum_vol, cur_high, cur_low, bar_open]
    """
    
    # #1. Unpack State
    cur_dollar = state[0]
    cur_vol = state[1]
    cur_high = state[2]
    cur_low = state[3]
    bar_open = state[4]
    
    n = len(prices)
    # Estimate output size (heuristic)
    est_size = int(n / 10) + 100 
    
    # #2. Initialize Output Arrays
    out_ts = np.zeros(est_size, dtype=np.int64)
    out_open = np.zeros(est_size, dtype=np.float64)
    out_high = np.zeros(est_size, dtype=np.float64)
    out_low = np.zeros(est_size, dtype=np.float64)
    out_close = np.zeros(est_size, dtype=np.float64)
    out_vol = np.zeros(est_size, dtype=np.float64)
    out_dollar = np.zeros(est_size, dtype=np.float64)
    
    bar_idx = 0
    
    # #3. Iterate Ticks
    for i in range(n):
        p = prices[i]
        q = quantities[i]
        val = p * q
        
        # Init new bar if coming from fresh state
        if cur_dollar == 0:
            bar_open = p
            cur_high = p
            cur_low = p
        else:
            if p > cur_high: cur_high = p
            if p < cur_low: cur_low = p
        
        cur_vol += q
        cur_dollar += val
        
        # #4. Check Threshold
        if cur_dollar >= threshold:
            # Commit Bar
            out_ts[bar_idx] = timestamps[i]
            out_open[bar_idx] = bar_open
            out_high[bar_idx] = cur_high
            out_low[bar_idx] = cur_low
            out_close[bar_idx] = p
            out_vol[bar_idx] = cur_vol
            out_dollar[bar_idx] = cur_dollar
            
            bar_idx += 1
            
            # Reset State
            cur_dollar = 0.0
            cur_vol = 0.0
            cur_high = -np.inf
            cur_low = np.inf
            
            # Resize logic if buffer full
            if bar_idx >= len(out_ts):
                new_size = len(out_ts) * 2
                
                new_ts = np.zeros(new_size, dtype=np.int64)
                new_ts[:len(out_ts)] = out_ts
                out_ts = new_ts
                
                new_o = np.zeros(new_size, dtype=np.float64)
                new_o[:len(out_open)] = out_open
                out_open = new_o
                
                new_h = np.zeros(new_size, dtype=np.float64)
                new_h[:len(out_high)] = out_high
                out_high = new_h

                new_l = np.zeros(new_size, dtype=np.float64)
                new_l[:len(out_low)] = out_low
                out_low = new_l

                new_c = np.zeros(new_size, dtype=np.float64)
                new_c[:len(out_close)] = out_close
                out_close = new_c
                
                new_v = np.zeros(new_size, dtype=np.float64)
                new_v[:len(out_vol)] = out_vol
                out_vol = new_v
                
                new_d = np.zeros(new_size, dtype=np.float64)
                new_d[:len(out_dollar)] = out_dollar
                out_dollar = new_d

    # #5. Pack Final State
    new_state = np.array([cur_dollar, cur_vol, cur_high, cur_low, bar_open], dtype=np.float64)
    
    return (
        out_ts[:bar_idx], 
        out_open[:bar_idx], 
        out_high[:bar_idx], 
        out_low[:bar_idx], 
        out_close[:bar_idx], 
        out_vol[:bar_idx], 
        out_dollar[:bar_idx],
        new_state
    )


#
# LOGIC BLOCK 3: MAIN EXECUTION
#

def run_test_compiler() -> None:
    print(f"--- Starting Test Dollar Bar Compiler ---")
    print(f"Input: {TestConfig.TRADES_PATH}")
    print(f"Output: {TestConfig.OUTPUT_ROOT}")
    
    # Start Timer
    start_time: float = time.time()
    
    # #1. Calculate ADV thresholds
    adv_series: pd.Series = get_rolling_adv(TestConfig.DOLLAR_BAR_WINDOW)
    
    if adv_series.empty:
        print("[ERR] Could not calculate ADV. Exiting.")
        return

    # #2. Locate AggTrade Files
    search_pattern: str = str(TestConfig.TRADES_PATH / "*.parquet")
    trade_files: List[str] = sorted(glob.glob(search_pattern))
    
    if not trade_files:
        print("[ERR] No AggTrade files found.")
        return
        
    print(f"Found {len(trade_files)} trade files.")

    # #3. Initialize State & Directories
    # Dictionary mapping target -> numpy state array
    states: Dict[int, np.ndarray] = {}
    output_dirs: Dict[int, Path] = {}
    
    for t in TestConfig.DOLLAR_BAR_TARGETS:
        # Create persistent state [cum_dollar, cum_vol, high, low, open]
        states[t] = np.array([0.0, 0.0, -np.inf, np.inf, 0.0], dtype=np.float64)
        
        # Create output dir: D:\rust_test\2000_bpd
        dir_path = TestConfig.OUTPUT_ROOT / f"{t}_bpd"
        os.makedirs(dir_path, exist_ok=True)
        output_dirs[t] = dir_path

    valid_start_date = adv_series.index[0]
    processed_count: int = 0

    # #4. Sequential Processing Loop
    for f in trade_files:
        try:
            # Extract date from filename: BTCUSDC_futures_aggTrades_2026-01-02.parquet
            date_part: str = Path(f).stem.split('_')[-1]
            file_date: pd.Timestamp = pd.to_datetime(date_part)
        except Exception:
            print(f"[WARN] Skipping file with bad name format: {f}")
            continue

        # Skip if we don't have ADV for this day
        day_key = file_date.floor('D')
        if day_key not in adv_series.index:
            # Only skip if it's BEFORE our valid data. 
            # If it's after, we might want to use the last known ADV, 
            # but strict logic says we skip to avoid bad thresholds.
            continue

        daily_adv: float = adv_series.loc[day_key]

        # Load Data
        try:
            df: pd.DataFrame = pd.read_parquet(f)
            # Normalize columns
            df.columns = [c.lower() for c in df.columns]
            
            # Extract arrays with explicit typing for Pylance/Numba compatibility
            ts_arr: np.ndarray = np.asarray(df['timestamp'], dtype=np.int64)
            price_arr: np.ndarray = np.asarray(df['price'], dtype=np.float64)
            qty_arr: np.ndarray = np.asarray(df['quantity'], dtype=np.float64)
        except Exception as e:
            print(f"[ERR] Reading {Path(f).name}: {e}")
            continue

        if len(ts_arr) == 0:
            continue

        summary: List[str] = []

        # #5. Process for each target
        for t in TestConfig.DOLLAR_BAR_TARGETS:
            threshold: float = daily_adv / t
            current_state: np.ndarray = states[t]
            
            (res_ts, res_o, res_h, res_l, res_c, res_v, res_d, new_state) = _process_day_chunk(
                ts_arr, price_arr, qty_arr, threshold, current_state
            )
            
            # Update state for next day
            states[t] = new_state
            
            if len(res_ts) > 0:
                # Create DataFrame
                bars_df = pd.DataFrame({
                    'timestamp': res_ts,
                    'open': res_o,
                    'high': res_h,
                    'low': res_l,
                    'close': res_c,
                    'volume': res_v,
                    'dollar_value': res_d
                })
                
                # Format: UTC Datetime Index
                bars_df['datetime'] = pd.to_datetime(bars_df['timestamp'], unit='ms')
                bars_df.set_index('datetime', inplace=True)
                
                # Save to specific target folder
                # Filename: BTCUSDC_futures_dollar_bars_2000pd_2026-01-02.parquet
                out_name = f"{TestConfig.SYMBOL}_{TestConfig.MARKET_TYPE}_dollar_bars_{t}pd_{date_part}.parquet"
                save_path = output_dirs[t] / out_name
                bars_df.to_parquet(save_path)
                
                summary.append(f"{t}pd:{len(bars_df)}")
            else:
                summary.append(f"{t}pd:0")

        processed_count += 1
        print(f"Processed {date_part} (ADV ${daily_adv:,.0f}) -> [{' | '.join(summary)}]")

    # End Timer
    end_time: float = time.time()
    elapsed_time: float = end_time - start_time
    
    print(f"\n[DONE] Successfully processed {processed_count} days.")
    print(f"Total Execution Time: {elapsed_time:.2f} seconds")


if __name__ == "__main__":
    run_test_compiler()