# @file: markowitz_app_streamlit.py
# @description: Interactive Streamlit dashboard for Markowitz Efficient Frontier with dynamic Asset Redistribution logic.
# @author: LAS.

import streamlit as st
import numpy as np
import pandas as pd
import plotly.graph_objects as go
from scipy.optimize import minimize
import sys
import os

#
# CONSTANTS & CONFIGURATION
#
st.set_page_config(layout="wide", page_title="Quant Portfolio Optimizer")

DEFAULT_RISK_FREE: float = 0.105
DEFAULT_POINTS: int = 50
DEFAULT_SEED: int = 42
DEFAULT_MAX_ALLOC: float = 0.99 

#
# DATA GENERATION LOGIC
#

def generate_asset_block(
    names: list[str], 
    base_ret: float, 
    base_vol: float, 
    ret_spread: float, 
    vol_spread: float,
    seed: int
) -> tuple[np.ndarray, np.ndarray]:
    """
    Helper to generate randomized but realistic stats for a group of assets.
    """
    # 1. Set seed and count
    np.random.seed(seed)
    count: int = len(names)
    
    # 2. Randomize around the base mean/vol
    returns: np.ndarray = np.random.uniform(base_ret - ret_spread, base_ret + ret_spread, count)
    vols: np.ndarray = np.random.uniform(base_vol - vol_spread, base_vol + vol_spread, count)
    
    return returns, vols

@st.cache_data
def get_analyst_expectations(seed: int) -> tuple[pd.Series, pd.DataFrame, dict]:
    """
    Returns realistic expected returns and covariance matrix for 50 assets.
    Cached for performance.
    """
    # 1. Define Asset Groups
    br_stocks: list[str] = ['VALE3', 'PETR4', 'ITUB4', 'B3SA3', 'BBAS3', 'ABEV3', 'WEGE3', 'RENT3', 'SUZB3', 'GGBR4', 
                 'RAIL3', 'JBSS3', 'LREN3', 'ELET3', 'CSAN3', 'PRIO3', 'RDOR3', 'VIVT3', 'MGLU3', 'HAPV3']
    br_fixed: list[str] = ['Tesouro_Selic', 'CDB_BancoMaster', 'FII_Papel']
    intl_assets: list[str] = ['IVVB11 (S&P500)', 'NASD11 (Nasdaq)', 'EURP11 (Europe)', 'GOLD11 (Gold)', 'US_Treasury',
                   'BDR_AAPL', 'BDR_MSFT', 'BDR_NVDA', 'BDR_GOOGL', 'BDR_AMZN', 'BDR_TSLA', 'BDR_META', 
                   'BDR_JPM', 'BDR_JNJ', 'BDR_XOM']
    crypto_assets: list[str] = ['BTC', 'ETH', 'SOL', 'BNB', 'XRP', 'ADA', 'DOGE', 'AVAX', 'DOT', 'TRX', 'LINK', 'MATIC']
    
    all_assets: list[str] = br_stocks + br_fixed + intl_assets + crypto_assets
    n_total: int = len(all_assets)
    
    asset_types: dict = {}
    for a in br_stocks: asset_types[a] = 'BR Equity'
    for a in br_fixed: asset_types[a] = 'BR Fixed Income'
    for a in intl_assets: asset_types[a] = 'International'
    for a in crypto_assets: asset_types[a] = 'Crypto'

    # 2. Generate Statistics
    r_stk, v_stk = generate_asset_block(br_stocks, 0.16, 0.30, 0.08, 0.15, seed)
    r_fix, v_fix = generate_asset_block(br_fixed, 0.125, 0.06, 0.02, 0.04, seed+1)
    r_int, v_int = generate_asset_block(intl_assets, 0.18, 0.20, 0.06, 0.10, seed+2)
    r_cry, v_cry = generate_asset_block(crypto_assets, 0.30, 0.50, 0.15, 0.25, seed+3)
    
    exp_returns: np.ndarray = np.concatenate([r_stk, r_fix, r_int, r_cry])
    volatilities: np.ndarray = np.concatenate([v_stk, v_fix, v_int, v_cry])
    
    # 3. Correlation Matrix
    corr_matrix: np.ndarray = np.eye(n_total)
    
    # Slices
    idx_stk: slice = slice(0, 20)
    idx_fix: slice = slice(20, 23)
    idx_int: slice = slice(23, 38)
    idx_cry: slice = slice(38, 50)
    
    # Intra-Group
    corr_matrix[idx_stk, idx_stk] = 0.55
    corr_matrix[idx_fix, idx_fix] = 0.85
    corr_matrix[idx_int, idx_int] = 0.65
    corr_matrix[idx_cry, idx_cry] = 0.75
    np.fill_diagonal(corr_matrix, 1.0)
    
    # Inter-Group
    corr_matrix[idx_stk, idx_fix] = 0.15; corr_matrix[idx_fix, idx_stk] = 0.15
    corr_matrix[idx_stk, idx_int] = 0.25; corr_matrix[idx_int, idx_stk] = 0.25
    corr_matrix[idx_stk, idx_cry] = 0.35; corr_matrix[idx_cry, idx_stk] = 0.35
    corr_matrix[idx_fix, idx_int] = 0.05; corr_matrix[idx_int, idx_fix] = 0.05
    corr_matrix[idx_fix, idx_cry] = 0.02; corr_matrix[idx_cry, idx_fix] = 0.02
    corr_matrix[idx_int, idx_cry] = 0.40; corr_matrix[idx_cry, idx_int] = 0.40
    
    # 4. Covariance
    outer_vol: np.ndarray = np.outer(volatilities, volatilities)
    cov_matrix_vals: np.ndarray = outer_vol * corr_matrix
    
    return (
        pd.Series(exp_returns, index=all_assets), 
        pd.DataFrame(cov_matrix_vals, index=all_assets, columns=all_assets),
        asset_types
    )

#
# OPTIMIZATION ENGINES
#

def get_portfolio_metrics(weights: np.ndarray, mean_returns: np.ndarray, cov_matrix: np.ndarray, risk_free: float) -> tuple[float, float, float]:
    ret: float = float(np.sum(mean_returns * weights))
    vol: float = float(np.sqrt(np.dot(weights.T, np.dot(cov_matrix, weights))))
    sharpe: float = (ret - risk_free) / vol if vol > 0 else 0.0
    return ret, vol, sharpe

def get_diversification_ratio(weights: np.ndarray, cov_matrix: np.ndarray) -> float:
    asset_vols: np.ndarray = np.sqrt(np.diag(cov_matrix))
    weighted_avg_vol: float = float(np.sum(weights * asset_vols))
    port_vol: float = float(np.sqrt(np.dot(weights.T, np.dot(cov_matrix, weights))))
    return weighted_avg_vol / port_vol if port_vol > 0 else 0.0

def minimize_volatility(weights: np.ndarray, cov_matrix: np.ndarray) -> float:
    return np.dot(weights.T, np.dot(cov_matrix, weights))

def negative_sharpe(weights: np.ndarray, mean_returns: np.ndarray, cov_matrix: np.ndarray, risk_free: float) -> float:
    ret, vol, _ = get_portfolio_metrics(weights, mean_returns, cov_matrix, risk_free)
    return -1 * (ret - risk_free) / vol if vol > 0 else 0.0

def negative_diversification_ratio(weights: np.ndarray, cov_matrix: np.ndarray) -> float:
    return -get_diversification_ratio(weights, cov_matrix)

@st.cache_data
def run_optimization(mean_returns: pd.Series, cov_matrix: pd.DataFrame, risk_free: float, max_alloc: float):
    """
    Runs SLSQP optimization. Cached by Streamlit to avoid re-running on every slider change.
    """
    num_assets: int = len(mean_returns)
    cov_matrix_np: np.ndarray = cov_matrix.to_numpy()
    mean_returns_np: np.ndarray = mean_returns.to_numpy()
    
    constraints: dict = {'type': 'eq', 'fun': lambda x: np.sum(x) - 1}
    bounds: tuple = tuple((0.0, max_alloc) for _ in range(num_assets))
    init_guess: np.ndarray = np.full(num_assets, 1.0 / num_assets)

    # 1. Tangency
    res_sharpe = minimize(negative_sharpe, init_guess, args=(mean_returns_np, cov_matrix_np, risk_free),
                          method='SLSQP', bounds=bounds, constraints=constraints)
    w_tangency: pd.Series = pd.Series(res_sharpe.x, index=mean_returns.index)

    # 2. MDP
    res_mdp = minimize(negative_diversification_ratio, init_guess, args=(cov_matrix_np,),
                       method='SLSQP', bounds=bounds, constraints=constraints)
    w_mdp: pd.Series = pd.Series(res_mdp.x, index=mean_returns.index)

    # 3. GMV
    res_gmv = minimize(minimize_volatility, init_guess, args=(cov_matrix_np,),
                       method='SLSQP', bounds=bounds, constraints=constraints)
    w_gmv: pd.Series = pd.Series(res_gmv.x, index=mean_returns.index)

    # 4. Frontier Line (and Store Weights for lookup)
    frontier_vol: list[float] = []
    frontier_ret: list[float] = []
    frontier_weights: list[np.ndarray] = []
    
    # Calculate returns range for frontier scan
    min_ret: float = get_portfolio_metrics(w_gmv.values, mean_returns_np, cov_matrix_np, risk_free)[0]
    max_ret: float = mean_returns.max()
    
    # Generate points from GMV return up to Max return
    target_returns: np.ndarray = np.linspace(min_ret, max_ret * 0.99, 100)

    for target in target_returns:
        cons_iter: tuple = (
            {'type': 'eq', 'fun': lambda x: np.sum(x) - 1},
            {'type': 'eq', 'fun': lambda x: np.sum(x * mean_returns_np) - target}
        )
        res = minimize(minimize_volatility, init_guess, args=(cov_matrix_np,),
                       method='SLSQP', bounds=bounds, constraints=cons_iter)
        if res.success:
            frontier_vol.append(np.sqrt(res.fun))
            frontier_ret.append(target)
            frontier_weights.append(res.x)
            
    df_frontier: pd.DataFrame = pd.DataFrame({'Returns': frontier_ret, 'Volatility': frontier_vol})
    df_weights_frontier: pd.DataFrame = pd.DataFrame(frontier_weights, columns=mean_returns.index)
    
    return df_frontier, w_tangency, w_mdp, w_gmv, df_weights_frontier

#
# UI & PLOTTING LOGIC
#

def main() -> None:
    st.sidebar.header("⚙️ Simulation Settings")
    risk_free_rate: float = st.sidebar.number_input("Risk Free Rate", 0.0, 0.20, DEFAULT_RISK_FREE, 0.005)
    max_alloc: float = st.sidebar.slider("Max Asset Allocation", 0.05, 1.0, DEFAULT_MAX_ALLOC, 0.01)
    seed: int = st.sidebar.number_input("Random Seed", 1, 1000, DEFAULT_SEED)
    
    # STRATEGY SELECTOR
    st.sidebar.markdown("---")
    st.sidebar.header("🎯 Benchmark Strategy")
    strategy_mode: str = st.sidebar.selectbox(
        "Choose Anchor Portfolio:",
        ["Max Sharpe (CML)", "Max Diversification (MDP)", "Global Min Var (GMV)"]
    )
    
    st.title("📈 Efficient Frontier & Dynamic Capital Allocation")
    st.markdown(f"""
    **Interactive Strategy:**
    * **Anchor Point:** {strategy_mode}
    * **Left of Anchor:** Capital Allocation Line (Mix of Cash + Chosen Portfolio).
    * **Right of Anchor:** Efficient Frontier Curve (Aggressive Asset Redistribution).
    """)

    # --- Run Calculation ---
    with st.spinner("Optimizing Portfolios..."):
        means, covs, types = get_analyst_expectations(seed)
        frontier, w_tan, w_mdp, w_gmv, weights_curve = run_optimization(means, covs, risk_free_rate, max_alloc)

    # --- Metrics Calculation ---
    t_ret, t_vol, t_sharpe = get_portfolio_metrics(w_tan.values, means.to_numpy(), covs.to_numpy(), risk_free_rate)
    m_ret, m_vol, m_sharpe = get_portfolio_metrics(w_mdp.values, means.to_numpy(), covs.to_numpy(), risk_free_rate)
    g_ret, g_vol, g_sharpe = get_portfolio_metrics(w_gmv.values, means.to_numpy(), covs.to_numpy(), risk_free_rate)

    # --- SET REFERENCE PORTFOLIO BASED ON SELECTION ---
    ref_vol: float = 0.0
    ref_ret: float = 0.0
    ref_weights: pd.Series = pd.Series()
    
    if "Max Sharpe" in strategy_mode:
        ref_vol, ref_ret, ref_weights = t_vol, t_ret, w_tan
    elif "Max Diversification" in strategy_mode:
        ref_vol, ref_ret, ref_weights = m_vol, m_ret, w_mdp
    elif "Global Min Var" in strategy_mode:
        ref_vol, ref_ret, ref_weights = g_vol, g_ret, w_gmv

    # --- DYNAMIC ANALYSIS (Control Logic) ---
    col1, col2 = st.columns([1, 2])
    
    min_slider_vol: float = 0.0
    max_slider_vol: float = float(frontier['Volatility'].max())
    
    with col1:
        st.info(f"Adjust Volatility relative to {strategy_mode}.")
        target_vol: float = st.slider("Target Volatility", min_slider_vol, max_slider_vol, ref_vol, 0.001)
        
        weight_cash: float = 0.0
        expected_return: float = 0.0
        implied_vol: float = 0.0
        chosen_weights: pd.Series = pd.Series()
        
        if target_vol < ref_vol:
            # LEFT OF ANCHOR (CAL Logic)
            weight_risky: float = target_vol / ref_vol if ref_vol > 0 else 0.0
            weight_cash = 1.0 - weight_risky
            
            # Linear Interpolation
            # Slope = (Ref_Ret - Rf) / Ref_Vol
            slope: float = (ref_ret - risk_free_rate) / ref_vol if ref_vol > 0 else 0.0
            expected_return = risk_free_rate + (slope * target_vol)
            implied_vol = target_vol
            chosen_weights = ref_weights * weight_risky
            
            st.success(f"🛡️ DEFENSIVE: Mixing Cash + {strategy_mode}.")
            
        else:
            # RIGHT OF ANCHOR (Frontier Logic)
            # Find nearest point on frontier
            idx_closest: int = int((frontier['Volatility'] - target_vol).abs().idxmin())
            
            expected_return = float(frontier.loc[idx_closest, 'Returns'])
            implied_vol = float(frontier.loc[idx_closest, 'Volatility'])
            chosen_weights = weights_curve.iloc[idx_closest]
            st.warning("🔥 AGGRESSIVE: Sliding up the Efficient Frontier.")

        st.metric("Expected Return", f"{expected_return:.2%}")
        st.metric("Actual Volatility", f"{implied_vol:.2%}")
        st.metric("Cash Allocation", f"{weight_cash:.1%}")

    # --- PLOTLY CHART ---
    fig = go.Figure()

    # 1. Frontier Line
    fig.add_trace(go.Scatter(
        x=frontier['Volatility'], y=frontier['Returns'],
        mode='lines', name='Efficient Frontier',
        line=dict(color='silver', width=3, dash='dash')
    ))

    # 2. Dynamic CAL Line (Connects Risk-Free to Selected Anchor)
    max_x_line: float = max(frontier['Volatility'].max(), ref_vol) * 1.3
    # Equation: y = mx + c => y = slope * x + risk_free
    current_slope: float = (ref_ret - risk_free_rate) / ref_vol if ref_vol > 0 else 0.0
    
    x_cal: list[float] = [0, max_x_line]
    y_cal: list[float] = [risk_free_rate, risk_free_rate + current_slope * max_x_line]
    
    fig.add_trace(go.Scatter(
        x=x_cal, y=y_cal,
        mode='lines', name='Capital Allocation Line',
        line=dict(color='red', width=2, dash='dot')
    ))

    # 3. Static Points (Tangency, MDP, GMV)
    fig.add_trace(go.Scatter(
        x=[t_vol], y=[t_ret], mode='markers', name='Tangency',
        marker=dict(symbol='x', size=12, color='gold', line=dict(color='white', width=1)),
        hovertemplate="<b>Tangency</b><br>Ret: %{y:.2%}<br>Vol: %{x:.2%}<extra></extra>"
    ))
    fig.add_trace(go.Scatter(
        x=[m_vol], y=[m_ret], mode='markers', name='Max Diversification',
        marker=dict(symbol='x', size=12, color='cyan', line=dict(color='white', width=1)),
        hovertemplate="<b>Max Div</b><br>Ret: %{y:.2%}<br>Vol: %{x:.2%}<extra></extra>"
    ))
    fig.add_trace(go.Scatter(
        x=[g_vol], y=[g_ret], mode='markers', name='Global Min Var',
        marker=dict(symbol='x', size=12, color='lime', line=dict(color='white', width=1)),
        hovertemplate="<b>GMV</b><br>Ret: %{y:.2%}<br>Vol: %{x:.2%}<extra></extra>"
    ))

    # 4. Asset Scatter
    asset_vols: np.ndarray = np.sqrt(np.diag(covs))
    colors_map: dict = {'BR Equity': '#1f77b4', 'BR Fixed Income': '#2ca02c', 'International': '#d62728', 'Crypto': '#9467bd'}
    
    # Combine interest based on the active strategy + frontier
    combined_interest: np.ndarray = np.maximum.reduce([w_tan.values, w_mdp.values, w_gmv.values])
    
    unique_types: list[str] = list(set(types.values()))
    for u_type in unique_types:
        indices: list[int] = [i for i, name in enumerate(means.index) if types.get(name) == u_type]
        if not indices: continue
        idx_arr: np.ndarray = np.array(indices)
        is_relevant: np.ndarray = combined_interest[idx_arr] > 0.001
        
        # Relevant Assets
        if is_relevant.any():
            chosen_idx = idx_arr[is_relevant]
            fig.add_trace(go.Scatter(
                x=asset_vols[chosen_idx], y=means.iloc[chosen_idx],
                mode='markers', name=u_type,
                marker=dict(size=10, color=colors_map.get(u_type, 'gray'), opacity=0.9, line=dict(width=1, color='white')),
                text=means.index[chosen_idx],
                hovertemplate="<b>%{text}</b><br>Ret: %{y:.2%}<br>Vol: %{x:.2%}<extra></extra>"
            ))
        
        # Ignored Assets (Faded)
        if (~is_relevant).any():
            ignored_idx = idx_arr[~is_relevant]
            fig.add_trace(go.Scatter(
                x=asset_vols[ignored_idx], y=means.iloc[ignored_idx],
                mode='markers', name=f"{u_type} (Ignored)",
                marker=dict(size=6, color=colors_map.get(u_type, 'gray'), opacity=0.2),
                text=means.index[ignored_idx],
                hovertemplate="<b>%{text}</b><br>Ret: %{y:.2%}<br>Vol: %{x:.2%}<extra></extra>",
                showlegend=False
            ))

    # 5. DYNAMIC POINT (You Are Here)
    fig.add_trace(go.Scatter(
        x=[implied_vol], y=[expected_return],
        mode='markers', name='Your Portfolio',
        marker=dict(size=18, color='white', line=dict(width=3, color='black')),
        hovertemplate="<b>Your Portfolio</b><br>Ret: %{y:.2%}<br>Vol: %{x:.2%}<extra></extra>"
    ))

    # Layout config
    fig.update_layout(
        title="Markowitz Efficient Frontier",
        xaxis_title="Volatility (Risk)",
        yaxis_title="Return",
        xaxis=dict(range=[0, max_x_line], showgrid=True),
        yaxis=dict(range=[0, max(max(y_cal), means.max()) * 1.1], showgrid=True),
        legend=dict(yanchor="bottom", y=0.01, xanchor="right", x=0.99),
        height=600,
        margin=dict(l=40, r=40, t=40, b=40),
        hovermode="closest"
    )

    st.plotly_chart(fig, use_container_width=True)

    # --- TABLE DISPLAY ---
    with col2:
        st.subheader("Asset Allocation")
        
        # Prepare Data
        sig_weights: pd.Series = chosen_weights[chosen_weights > 0.001]
        data: list[dict] = []
        if weight_cash > 0.001:
            data.append({"Asset": "CASH / RISK-FREE", "Type": "Cash", "Weight": weight_cash})
            
        for asset, w in sig_weights.items():
            data.append({
                "Asset": asset,
                "Type": types.get(asset, "Unknown"),
                "Weight": float(w)
            })
            
        df_comp: pd.DataFrame = pd.DataFrame(data)
        
        if not df_comp.empty:
            df_comp['Sort_W'] = df_comp['Weight'].round(4)
            df_comp = df_comp.sort_values(by=['Sort_W', 'Asset'], ascending=[False, True]).drop(columns=['Sort_W'])
            df_comp['Weight'] = df_comp['Weight'].apply(lambda x: f"{x:.1%}")
            
            st.dataframe(df_comp, use_container_width=True, hide_index=True)
        else:
            st.write("No assets allocated.")

if __name__ == "__main__":
    # Execution Guard
    try:
        from streamlit.runtime.scriptrunner import get_script_run_ctx
        if not get_script_run_ctx():
            print("\n" + "="*60)
            print("🛑 STREAMLIT EXECUTION ERROR")
            print("="*60)
            print("You are running this script as a standard Python file.")
            print("Streamlit apps must be run using the 'streamlit run' command.")
            print(f"\n👉 Run this command in your terminal:\n")
            print(f"   streamlit run {os.path.basename(sys.argv[0])}")
            print("\n" + "="*60)
    except ImportError:
        pass
        
    main()