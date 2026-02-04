# React

1. Use "strict mode"



# ingestion_engine

## models.rs
> 1.
    Heap Allocation:
        Stores the ticker (e.g., "BTCUSDT"). Note: A String involves a heap allocation. In ultra-low latency systems, you might eventually replace this with a SmallVec or a static &'static str to avoid memory overhead.
    
    pub struct OrderBook {
        pub symbol: String,

> 2.
    Scenarios for Local Copies (Arc Immutability)

    Scenario A: Order Execution Simulation ("What-If" Analysis)
        >> The Problem: Your trading bot wants to know: "If I place a market buy for 50 BTC right now, what will my average entry price be?"
        >> Why Arc fails: To calculate this, the bot effectively needs to "eat" through the order book levels logic-wise (simulating the removal of liquidity). You cannot remove items from the Arc bids vector because that would delete the liquidity for the Frontend user too.
        >> The Solution: The bot creates a local Clone of the vector. It performs the simulation on its private copy, destroying the levels as it calculates the fill price, while the Arc remains pristine for everyone else.

    Scenario B: filtering & Cleaning (Wash Trading / Spam)
        >> The Problem: You want to display a "Cleaned Book" that hides orders smaller than $100 (dust) or specific market maker IDs.
        >> Why Arc fails: You cannot filter the Arc list in place.
        >> The Solution: You iterate over the Arc list, and copy only the valid orders into a new Vec. This new vector is your "Cleaned View."


## binance.rs
> 1.
    hard-coded "@depth20" -> change to "depth: u32":
        >> backend option to change in the (config.toml ??)
        >> frontend option in the UI

> 2.
    "Simplified" Logic:
        >> Snapshot vs. Diff: The @depth20 stream sends a full picture of the top 20 levels every second. This is easy to handle (just replace the old list).
        >> True Depth: A professional "Full Depth" stream (@depth) sends diffs (e.g., "Change price 100.0 to quantity 5"). That requires complex logic to buffer events, check sequence IDs, and modify the existing book in memory. Our logic is "simplified" because it skips that complexity by using the snapshot stream.


## interfaces.rs
    Future Workflow (How to add a Strategy later):
        1. Create strategy.rs.
        2. Implement DataProcessor for your struct.
        3. In main.rs: add engine.register_processor(Box::new(MyStrategy)).await;.

# main.rs
>1.
    'tokio::spawn' vs 'pinned threads'
        >> 'tokio::spawn' for lots of symbols (50+)
        >> 'pinned threads' for specific symbols (a few, I'm not sure how many)