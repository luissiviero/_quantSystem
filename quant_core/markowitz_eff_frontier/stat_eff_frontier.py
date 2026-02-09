# @file: markowitz_optimization_slsqp.py
# @description: Solves Efficient Frontier, Tangency, MDP, and GMV portfolios for a 50-asset universe.
# @author: LAS.

import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import matplotlib.patheffects as pe
from scipy.optimize import minimize

#
# CONSTANTS & CONFIGURATION
#
RISK_FREE_RATE: float = 0.105  # 10.5% Selic
FRONTIER_POINTS: int = 50     # Number of points to calculate on the curve
SEED: int = 42
MAX_ASSET_ALLOCATION: float = 0.99  # Max 9% per asset constraint

#
# DATA INPUT LOGIC
#

def generate_asset_block(
    names: list[str], 
    base_ret: float, 
    base_vol: float, 
    ret_spread: float, 
    vol_spread: float
) -> tuple[np.ndarray, np.ndarray]:
    """
    Helper to generate randomized but realistic stats for a group of assets.
    """
    count: int = len(names)
    
    # Randomize around the base mean/vol
    returns: np.ndarray = np.random.uniform(base_ret - ret_spread, base_ret + ret_spread, count)
    vols: np.ndarray = np.random.uniform(base_vol - vol_spread, base_vol + vol_spread, count)
    
    return returns, vols


def get_analyst_expectations() -> tuple[pd.Series, pd.DataFrame, dict]:
    """
    Returns realistic expected returns and covariance matrix for 50 assets across 4 categories.
    Returns: (Returns Series, Covariance DataFrame, Asset Type Dictionary)
    """
    np.random.seed(SEED)
    
    # 1. Define Asset Groups (50 Assets Total)
    
    # Group A: Brazilian Stocks (20)
    br_stocks: list[str] = [
        'VALE3', 'PETR4', 'ITUB4', 'B3SA3', 'BBAS3', 'ABEV3', 'WEGE3', 'RENT3', 
        'SUZB3', 'GGBR4', 'RAIL3', 'JBSS3', 'LREN3', 'ELET3', 'CSAN3', 'PRIO3', 
        'RDOR3', 'VIVT3', 'MGLU3', 'HAPV3'
    ]
    
    # Group B: Brazilian Fixed Income (Reduced to 3)
    br_fixed: list[str] = [
        'Tesouro_Selic', 'CDB_BancoMaster', 'FII_Papel'
    ]
    
    # Group C: International Assets (15)
    intl_assets: list[str] = [
        'IVVB11 (S&P500)', 'NASD11 (Nasdaq)', 'EURP11 (Europe)', 'GOLD11 (Gold)', 'US_Treasury',
        'BDR_AAPL', 'BDR_MSFT', 'BDR_NVDA', 'BDR_GOOGL', 'BDR_AMZN',
        'BDR_TSLA', 'BDR_META', 'BDR_JPM', 'BDR_JNJ', 'BDR_XOM'
    ]

    # Group D: Crypto Assets (12)
    crypto_assets: list[str] = [
        'BTC', 'ETH', 'SOL', 'BNB', 'XRP', 'ADA', 
        'DOGE', 'AVAX', 'DOT', 'TRX', 'LINK', 'MATIC'
    ]
    
    all_assets: list[str] = br_stocks + br_fixed + intl_assets + crypto_assets
    n_total: int = len(all_assets)
    
    # Map for coloring later
    asset_types: dict = {}
    for a in br_stocks: asset_types[a] = 'BR Equity'
    for a in br_fixed: asset_types[a] = 'BR Fixed Income'
    for a in intl_assets: asset_types[a] = 'International'
    for a in crypto_assets: asset_types[a] = 'Crypto'


    # 2. Generate Statistics per Block
    
    # BR Stocks: Avg Ret 16%, Vol 30%
    r_stk, v_stk = generate_asset_block(br_stocks, 0.16, 0.30, 0.08, 0.15)
    
    # BR Fixed: Avg Ret 12.5%, Vol 6%
    r_fix, v_fix = generate_asset_block(br_fixed, 0.125, 0.06, 0.02, 0.04)
    
    # Intl: Avg Ret 18%, Vol 20%
    r_int, v_int = generate_asset_block(intl_assets, 0.18, 0.20, 0.06, 0.10)

    # Crypto: Avg Ret 30%, Vol 50%
    r_cry, v_cry = generate_asset_block(crypto_assets, 0.30, 0.50, 0.15, 0.25)
    
    exp_returns: np.ndarray = np.concatenate([r_stk, r_fix, r_int, r_cry])
    volatilities: np.ndarray = np.concatenate([v_stk, v_fix, v_int, v_cry])
    
    
    # 3. Construct Block Correlation Matrix
    
    # Initialize with identity
    corr_matrix: np.ndarray = np.eye(n_total)
    
    # Define indices
    idx_stk = slice(0, 20)
    idx_fix = slice(20, 23)
    idx_int = slice(23, 38)
    idx_cry = slice(38, 50)
    
    # --- Intra-Group Correlations ---
    corr_matrix[idx_stk, idx_stk] = 0.55
    corr_matrix[idx_fix, idx_fix] = 0.85
    corr_matrix[idx_int, idx_int] = 0.65
    corr_matrix[idx_cry, idx_cry] = 0.75
    
    np.fill_diagonal(corr_matrix, 1.0)
    
    # --- Inter-Group Correlations ---
    corr_matrix[idx_stk, idx_fix] = 0.15
    corr_matrix[idx_fix, idx_stk] = 0.15
    
    corr_matrix[idx_stk, idx_int] = 0.25
    corr_matrix[idx_int, idx_stk] = 0.25

    corr_matrix[idx_stk, idx_cry] = 0.35
    corr_matrix[idx_cry, idx_stk] = 0.35
    
    corr_matrix[idx_fix, idx_int] = 0.05
    corr_matrix[idx_int, idx_fix] = 0.05

    corr_matrix[idx_fix, idx_cry] = 0.02
    corr_matrix[idx_cry, idx_fix] = 0.02

    corr_matrix[idx_int, idx_cry] = 0.40
    corr_matrix[idx_cry, idx_int] = 0.40
    
    
    # 4. Convert to Covariance
    
    outer_vol: np.ndarray = np.outer(volatilities, volatilities)
    cov_matrix_vals: np.ndarray = outer_vol * corr_matrix
    
    
    # 5. Package
    
    return (
        pd.Series(exp_returns, index=all_assets), 
        pd.DataFrame(cov_matrix_vals, index=all_assets, columns=all_assets),
        asset_types
    )


#
# OPTIMIZATION ENGINES (SLSQP)
#

def get_portfolio_metrics(weights: np.ndarray, mean_returns: np.ndarray, cov_matrix: np.ndarray) -> tuple[float, float, float]:
    """
    Helper to calculate Ret, Vol, Sharpe for a given weight set.
    """
    # Explicitly cast to float to satisfy strict type checkers
    ret: float = float(np.sum(mean_returns * weights))
    vol: float = float(np.sqrt(np.dot(weights.T, np.dot(cov_matrix, weights))))
    sharpe: float = (ret - RISK_FREE_RATE) / vol
    return ret, vol, sharpe


def get_diversification_ratio(weights: np.ndarray, cov_matrix: np.ndarray) -> float:
    """
    Calculates the Diversification Ratio: (Weighted Avg Vol) / (Portfolio Vol).
    Higher is better.
    """
    asset_vols = np.sqrt(np.diag(cov_matrix))
    weighted_avg_vol = float(np.sum(weights * asset_vols))
    port_vol = float(np.sqrt(np.dot(weights.T, np.dot(cov_matrix, weights))))
    
    return weighted_avg_vol / port_vol


def minimize_volatility(weights: np.ndarray, cov_matrix: np.ndarray) -> float:
    """
    Objective Function: Minimize Variance (Vol^2).
    """
    return np.dot(weights.T, np.dot(cov_matrix, weights))


def negative_sharpe(weights: np.ndarray, mean_returns: np.ndarray, cov_matrix: np.ndarray) -> float:
    """
    Objective Function: Minimize Negative Sharpe (Maximize Sharpe).
    """
    ret, vol, _ = get_portfolio_metrics(weights, mean_returns, cov_matrix)
    return -1 * (ret - RISK_FREE_RATE) / vol


def negative_diversification_ratio(weights: np.ndarray, cov_matrix: np.ndarray) -> float:
    """
    Objective Function: Minimize Negative Div Ratio (Maximize Div Ratio).
    """
    return -get_diversification_ratio(weights, cov_matrix)


def run_optimization(mean_returns: pd.Series, cov_matrix: pd.DataFrame) -> tuple[pd.DataFrame, pd.Series, pd.Series, pd.Series]:
    """
    Uses SLSQP to find Efficient Frontier, Tangency, Max Diversification, and GMV portfolios.
    """
    
    num_assets: int = len(mean_returns)
    # Using to_numpy() for strict type compatibility
    cov_matrix_np = cov_matrix.to_numpy()
    mean_returns_np = mean_returns.to_numpy()
    
    args_sharpe: tuple = (mean_returns_np, cov_matrix_np)
    args_div: tuple = (cov_matrix_np,) # Div ratio doesn't need returns
    args_gmv: tuple = (cov_matrix_np,)
    
    #1. Constraint: Sum of weights = 1
    constraints: dict = {'type': 'eq', 'fun': lambda x: np.sum(x) - 1}
    
    #2. Bounds: 0 <= weight <= MAX_ASSET_ALLOCATION
    bounds: tuple = tuple((0.0, MAX_ASSET_ALLOCATION) for asset in range(num_assets))
    
    #3. Initial Guess
    init_guess: np.ndarray = np.full(num_assets, 1.0 / num_assets)


    #4. SOLVE FOR TANGENCY PORTFOLIO (Max Sharpe)
    print(f"Optimizing for Max Sharpe Ratio...")
    result_max_sharpe = minimize(
        negative_sharpe,
        init_guess,
        args=args_sharpe,
        method='SLSQP',
        bounds=bounds,
        constraints=constraints
    )
    tangency_weights: pd.Series = pd.Series(result_max_sharpe.x, index=mean_returns.index)


    #5. SOLVE FOR MAX DIVERSIFICATION PORTFOLIO (MDP)
    print(f"Optimizing for Max Diversification Ratio...")
    result_mdp = minimize(
        negative_diversification_ratio,
        init_guess,
        args=args_div,
        method='SLSQP',
        bounds=bounds,
        constraints=constraints
    )
    mdp_weights: pd.Series = pd.Series(result_mdp.x, index=mean_returns.index)


    #6. SOLVE FOR GLOBAL MINIMUM VARIANCE (GMV)
    print(f"Optimizing for Global Minimum Variance (GMV)...")
    result_gmv = minimize(
        minimize_volatility,
        init_guess,
        args=args_gmv,
        method='SLSQP',
        bounds=bounds,
        constraints=constraints
    )
    gmv_weights: pd.Series = pd.Series(result_gmv.x, index=mean_returns.index)


    #7. SOLVE FOR EFFICIENT FRONTIER LINE
    print("Tracing Efficient Frontier...")
    min_ret = mean_returns.min()
    max_ret = mean_returns.max()
    target_returns: np.ndarray = np.linspace(min_ret, max_ret * 0.95, FRONTIER_POINTS)
    
    frontier_vol: list[float] = []
    frontier_ret: list[float] = []
    
    for target in target_returns:
        iter_constraints = (
            {'type': 'eq', 'fun': lambda x: np.sum(x) - 1},
            {'type': 'eq', 'fun': lambda x: np.sum(x * mean_returns_np) - target}
        )
        
        result = minimize(
            minimize_volatility,
            init_guess,
            args=(cov_matrix_np,),
            method='SLSQP',
            bounds=bounds,
            constraints=iter_constraints
        )
        
        if result.success:
            frontier_vol.append(np.sqrt(result.fun))
            frontier_ret.append(target)
            
    df_frontier: pd.DataFrame = pd.DataFrame({
        'Returns': frontier_ret,
        'Volatility': frontier_vol
    })
    
    return df_frontier, tangency_weights, mdp_weights, gmv_weights


#
# VISUALIZATION LOGIC
#

def plot_slsqp_results(
    df_frontier: pd.DataFrame, 
    tangency_weights: pd.Series, 
    mdp_weights: pd.Series,
    gmv_weights: pd.Series,
    mean_ret: pd.Series, 
    cov_mat: pd.DataFrame,
    asset_types: dict
) -> None:
    """
    Plots Frontier with Tangency (Gold), Max Div (Cyan), and GMV (Lime).
    """
    
    #1. Calculate Metrics
    
    # Tangency
    t_ret, t_vol, t_sharpe = get_portfolio_metrics(
        tangency_weights.values, mean_ret.to_numpy(), cov_mat.to_numpy()
    )
    
    # MDP
    mdp_ret, mdp_vol, mdp_sharpe = get_portfolio_metrics(
        mdp_weights.values, mean_ret.to_numpy(), cov_mat.to_numpy()
    )

    # GMV
    gmv_ret, gmv_vol, gmv_sharpe = get_portfolio_metrics(
        gmv_weights.values, mean_ret.to_numpy(), cov_mat.to_numpy()
    )
    
    
    #2. Setup Plot
    plt.figure(figsize=(14, 9))
    
    # Plot Frontier
    plt.plot(
        df_frontier['Volatility'], 
        df_frontier['Returns'], 
        'k--', 
        linewidth=2, 
        label='Efficient Frontier'
    )
    
    
    #3. Draw Capital Market Lines
    max_x_plot = max(df_frontier['Volatility'].max(), t_vol) * 1.5
    x_cml = np.linspace(0, max_x_plot, 100)
    
    # Standard CML (Tangency)
    y_cml = RISK_FREE_RATE + t_sharpe * x_cml
    plt.plot(x_cml, y_cml, color='red', linestyle='-.', linewidth=2, label='CML (Max Sharpe)')

    # MDP Strategic Line
    y_mdp_line = RISK_FREE_RATE + mdp_sharpe * x_cml
    plt.plot(x_cml, y_mdp_line, color='cyan', linestyle=':', linewidth=2, label='MDP Strategic Line')

    # GMV Strategic Line (Defensive)
    y_gmv_line = RISK_FREE_RATE + gmv_sharpe * x_cml
    plt.plot(x_cml, y_gmv_line, color='lime', linestyle=':', linewidth=2, label='GMV Strategic Line (Defensive)')


    #4. Plot Special Portfolios
    
    # Tangency
    plt.scatter(
        t_vol, t_ret, 
        color='gold', marker='X', s=150, zorder=20, edgecolor='black', linewidth=1.5, 
        label='Tangency'
    )
    
    # Max Diversification
    plt.scatter(
        mdp_vol, mdp_ret, 
        color='cyan', marker='X', s=150, zorder=20, edgecolor='black', linewidth=1.5, 
        label='Max Diversification'
    )

    # GMV
    plt.scatter(
        gmv_vol, gmv_ret, 
        color='lime', marker='X', s=150, zorder=20, edgecolor='black', linewidth=1.5, 
        label='Global Min Variance'
    )
    
    
    #5. Plot Assets & Labels
    asset_vols: np.ndarray = np.sqrt(np.diag(cov_mat))
    colors: dict = {
        'BR Equity': '#1f77b4', 'BR Fixed Income': '#2ca02c', 
        'International': '#d62728', 'Crypto': '#9467bd'
    }
    
    unique_types = list(set(asset_types.values()))
    WEIGHT_THRESHOLD: float = 0.001 
    
    labels_to_plot: list[dict] = []
    
    # Combined weights for highlighting logic (union of all three portfolios)
    combined_interest = np.maximum.reduce([
        tangency_weights.values, 
        mdp_weights.values, 
        gmv_weights.values
    ])

    for u_type in unique_types:
        indices = [i for i, name in enumerate(mean_ret.index) if asset_types.get(name) == u_type]
        if not indices: continue
        
        idx_arr = np.array(indices)
        
        # Check if asset is significant in ANY portfolio
        is_relevant = combined_interest[idx_arr] > WEIGHT_THRESHOLD
        
        # Plot Ignored
        if (~is_relevant).any():
            plt.scatter(
                asset_vols[idx_arr][~is_relevant], 
                mean_ret.iloc[idx_arr][~is_relevant], 
                color=colors.get(u_type, 'gray'), 
                marker='o', s=40, alpha=0.15, 
                label=u_type 
            )
            
        # Plot Chosen
        if is_relevant.any():
            chosen_indices = idx_arr[is_relevant]
            plt.scatter(
                asset_vols[chosen_indices], 
                mean_ret.iloc[chosen_indices], 
                color=colors.get(u_type, 'gray'), 
                marker='o', s=100, alpha=0.9, 
                edgecolor='black', linewidth=1, zorder=10
            )
            
            for i in chosen_indices:
                labels_to_plot.append({
                    'x': asset_vols[i], 'y': mean_ret.iloc[i], 'name': mean_ret.index[i]
                })

    # 6. Label Placement (Dodge)
    labels_to_plot.sort(key=lambda item: item['x'])
    placed_labels: list[dict] = []
    
    for item in labels_to_plot:
        x, y, name = item['x'], item['y'], item['name']
        xy_offset = [5, 5]
        count_collisions = 0
        
        for placed in placed_labels:
            dist = np.sqrt(((x - placed['x'])*2)**2 + ((y - placed['y'])*4)**2)
            if dist < 0.05: count_collisions += 1
        
        if count_collisions > 0:
            if count_collisions % 2 == 1: xy_offset[1] += 15 * ((count_collisions + 1) // 2)
            else: xy_offset[1] -= 15 * (count_collisions // 2)
            xy_offset[0] += 5 * count_collisions

        plt.annotate(
            name, (x, y), xytext=xy_offset, textcoords='offset points',
            fontsize=7, fontweight='bold', zorder=15,
            path_effects=[pe.withStroke(linewidth=2, foreground="white")]
        )
        placed_labels.append(item)

    
    #7. Formatting
    plt.title(f'Efficient Frontier: Tangency vs. MDP vs. GMV\n(Constraint: Max {MAX_ASSET_ALLOCATION:.0%} per asset)')
    plt.xlabel('Volatility (Risk)')
    plt.ylabel('Expected Return')
    
    handles, labels = plt.gca().get_legend_handles_labels()
    by_label = dict(zip(labels, handles))
    plt.legend(by_label.values(), by_label.keys(), loc='lower right')
    
    plt.grid(True, linestyle='--', alpha=0.5)
    plt.xlim(0, max_x_plot)
    plt.ylim(0, max(y_cml.max(), mean_ret.max()) * 1.1)

    
#8. Text Output
    
    def print_top_holdings(title: str, weights: pd.Series) -> None:
        print(f"\n--- {title} (Top Holdings) ---")
        
        # 1. Filter significant weights
        sig: pd.Series = weights[weights > 0.001]
        
        # 2. Convert to DataFrame for multi-column sorting
        # Reset index to treat asset names as a column
        df_sig: pd.DataFrame = sig.reset_index()
        df_sig.columns = ['asset', 'weight'] # Rename for clarity
        
        # 3. Create a rounded column for sorting stability (handles float precision issues)
        df_sig['sort_weight'] = df_sig['weight'].round(5)
        
        # 4. Sort by Weight (Desc) then Name (Asc)
        df_sig = df_sig.sort_values(by=['sort_weight', 'asset'], ascending=[False, True])
        
        # 5. Print top 10
        for i, row in enumerate(df_sig.itertuples(index=False)):
            if i >= 10: break
            # Access fields from named tuple
            asset_name: str = str(row.asset)
            weight_val: float = float(row.weight)
            
            a_type: str = asset_types.get(asset_name, "Unknown")
            print(f"{asset_name:<20} ({a_type:<15}): {weight_val:.1%}")

    print_top_holdings("TANGENCY PORTFOLIO (Max Sharpe)", tangency_weights)
    print_top_holdings("MAX DIVERSIFICATION (Robust)", mdp_weights)
    print_top_holdings("GLOBAL MIN VARIANCE (Defensive)", gmv_weights)
    
    print("\n[INFO] Displaying Chart. Close the plot window to finish.")
    try:
        plt.show()
    except KeyboardInterrupt:
        print("\n[INFO] Window closed.")
    except Exception as e:
        print(f"\n[Warning] Matplotlib backend error: {e}")


if __name__ == "__main__":
    means, covs, types = get_analyst_expectations()
    frontier_line, optimal_w, mdp_w, gmv_w = run_optimization(means, covs)
    plot_slsqp_results(frontier_line, optimal_w, mdp_w, gmv_w, means, covs, types)