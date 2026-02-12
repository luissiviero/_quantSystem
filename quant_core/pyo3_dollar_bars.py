# @file: test_dollar_bars_rust.py
# @description: Standalone script to compile Dollar Bars using Rust (PyO3).
# @author: v5 helper

import os
import glob
import sys
import time
import pandas as pd
import numpy as np
from pathlib import Path
from datetime import datetime
from typing import List, Dict, Tuple, Any

# Import the compiled Rust module
# Ensure 'maturin develop' was run in quant_core/rust_core
try:
    import rust_core
except ImportError:
    print("CRITICAL ERROR: Could not import 'rust_core'.")
    print("Please run 'maturin develop' inside 'quant_core/rust_core' first.")
    sys.exit(1)

#
# CONSTANTS & CONFIGURATION
#

class TestConfig:
    # Input Paths
    BASE_PATH: Path = Path(r"D:\v5Protocol\binance\BTCUSDC\futures")
    KLINES_PATH: Path = BASE_PATH / "klines" / "1d"
    TRADES_PATH: Path = BASE_PATH / "aggtrades"
    
    # Output Path
    OUTPUT_ROOT: Path = Path(r"D:\dollar_bar_test\rust")
    
    # Algorithm Settings
    DOLLAR_BAR_TARGETS: List[int] = [10000, 4000, 2000, 1000, 500, 200]
    DOLLAR_BAR_WINDOW: int = 30
    
    # File Patterns
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
    daily_vol: pd.Series = full_df['QuoteVolume'].resample('D').sum()
    rolling_adv: pd.Series = daily_vol.rolling(window=window).mean()
    
    # Shift by 1
    prior_adv: pd.Series = rolling_adv.shift(1).dropna()
    
    print(f"   [ADV] Ready. Coverage: {len(prior_adv)} days.")
    return prior_adv


#
# LOGIC BLOCK 2: RUST WRAPPER
#

def run_rust_optimization() -> None:
    print(f"--- Starting Dollar Bar Compiler (Rust Optimized) ---")
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
    states: Dict[int, np.ndarray] = {}
    output_dirs: Dict[int, Path] = {}
    
    for t in TestConfig.DOLLAR_BAR_TARGETS:
        # State: [cum_dollar, cum_vol, high, low, open]
        states[t] = np.array([0.0, 0.0, -np.inf, np.inf, 0.0], dtype=np.float64)
        
        dir_path = TestConfig.OUTPUT_ROOT / f"{t}_bpd"
        os.makedirs(dir_path, exist_ok=True)
        output_dirs[t] = dir_path

    processed_count: int = 0

    # #4. Sequential Processing Loop
    for f in trade_files:
        try:
            date_part: str = Path(f).stem.split('_')[-1]
            file_date: pd.Timestamp = pd.to_datetime(date_part)
        except Exception:
            continue

        day_key = file_date.floor('D')
        if day_key not in adv_series.index:
            continue

        daily_adv: float = adv_series.loc[day_key]

        # Load Data
        try:
            df: pd.DataFrame = pd.read_parquet(f)
            df.columns = [c.lower() for c in df.columns]
            
            # MUST be contiguous arrays for Rust PyO3
            ts_arr = np.ascontiguousarray(df['timestamp'].values, dtype=np.int64)
            price_arr = np.ascontiguousarray(df['price'].values, dtype=np.float64)
            qty_arr = np.ascontiguousarray(df['quantity'].values, dtype=np.float64)
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
            
            # CALL RUST FUNCTION
            # Note: We must pass float64 for threshold
            (res_ts, res_o, res_h, res_l, res_c, res_v, res_d, new_state) = rust_core.process_dollar_bar_chunk( # type: ignore
                ts_arr, 
                price_arr, 
                qty_arr, 
                float(threshold), 
                current_state
            )
            
            states[t] = new_state
            
            if len(res_ts) > 0:
                bars_df = pd.DataFrame({
                    'timestamp': res_ts,
                    'open': res_o,
                    'high': res_h,
                    'low': res_l,
                    'close': res_c,
                    'volume': res_v,
                    'dollar_value': res_d
                })
                
                bars_df['datetime'] = pd.to_datetime(bars_df['timestamp'], unit='ms')
                bars_df.set_index('datetime', inplace=True)
                
                out_name = f"{TestConfig.SYMBOL}_{TestConfig.MARKET_TYPE}_dollar_bars_{t}pd_{date_part}.parquet"
                save_path = output_dirs[t] / out_name
                bars_df.to_parquet(save_path)
                
                summary.append(f"{t}pd:{len(bars_df)}")
            else:
                summary.append(f"{t}pd:0")

        processed_count += 1
        print(f"Processed {date_part} (ADV ${daily_adv:,.0f}) -> [{' | '.join(summary)}]")

    end_time: float = time.time()
    elapsed_time: float = end_time - start_time
    
    print(f"\n[DONE] Successfully processed {processed_count} days.")
    print(f"Total Execution Time: {elapsed_time:.2f} seconds")


if __name__ == "__main__":
    run_rust_optimization()