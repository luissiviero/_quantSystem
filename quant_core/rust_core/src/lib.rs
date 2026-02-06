// @file: quant_core\rust_core\src\lib.rs
// @description: Optimized Dollar Bar generation using PyO3 and Numpy.
// @author: LAS.

use pyo3::prelude::*;
use numpy::{PyReadonlyArray1, PyArray1, IntoPyArray};

#[pyfunction]
#[allow(clippy::too_many_arguments)]
fn process_dollar_bar_chunk<'py>(
    py: Python<'py>,
    timestamps: PyReadonlyArray1<i64>,
    prices: PyReadonlyArray1<f64>,
    quantities: PyReadonlyArray1<f64>,
    threshold: f64,
    state: PyReadonlyArray1<f64>,
) -> PyResult<(
    &'py PyArray1<i64>,
    &'py PyArray1<f64>,
    &'py PyArray1<f64>,
    &'py PyArray1<f64>,
    &'py PyArray1<f64>,
    &'py PyArray1<f64>,
    &'py PyArray1<f64>,
    &'py PyArray1<f64>
)> {
    // #1. Access raw array views for max speed (no copying)
    let ts_in = timestamps.as_array();
    let price_in = prices.as_array();
    let qty_in = quantities.as_array();
    let state_in = state.as_array();

    // #2. Unpack State
    let mut cur_dollar = state_in[0];
    let mut cur_vol = state_in[1];
    let mut cur_high = state_in[2];
    let mut cur_low = state_in[3];
    let mut bar_open = state_in[4];

    let n = price_in.len();
    
    // Pre-allocate vectors
    let est_cap = n / 10; 
    let mut out_ts = Vec::with_capacity(est_cap);
    let mut out_open = Vec::with_capacity(est_cap);
    let mut out_high = Vec::with_capacity(est_cap);
    let mut out_low = Vec::with_capacity(est_cap);
    let mut out_close = Vec::with_capacity(est_cap);
    let mut out_vol = Vec::with_capacity(est_cap);
    let mut out_dollar = Vec::with_capacity(est_cap);

    // #3. Core Processing Loop
    for i in 0..n {
        let p = price_in[i];
        let q = qty_in[i];
        let val = p * q;

        if cur_dollar == 0.0 {
            bar_open = p;
            cur_high = p;
            cur_low = p;
        } else {
            if p > cur_high { cur_high = p; }
            if p < cur_low { cur_low = p; }
        }

        cur_vol += q;
        cur_dollar += val;

        if cur_dollar >= threshold {
            out_ts.push(ts_in[i]);
            out_open.push(bar_open);
            out_high.push(cur_high);
            out_low.push(cur_low);
            out_close.push(p);
            out_vol.push(cur_vol);
            out_dollar.push(cur_dollar);

            cur_dollar = 0.0;
            cur_vol = 0.0;
            cur_high = f64::NEG_INFINITY;
            cur_low = f64::INFINITY;
        }
    }

    // #4. Pack Output State
    let new_state_vec = vec![cur_dollar, cur_vol, cur_high, cur_low, bar_open];
    let new_state_arr = new_state_vec.into_pyarray(py);

    // #5. Convert Vectors to Numpy Arrays
    Ok((
        out_ts.into_pyarray(py),
        out_open.into_pyarray(py),
        out_high.into_pyarray(py),
        out_low.into_pyarray(py),
        out_close.into_pyarray(py),
        out_vol.into_pyarray(py),
        out_dollar.into_pyarray(py),
        new_state_arr
    ))
}

#[pymodule]
fn rust_core(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(process_dollar_bar_chunk, m)?)?;
    Ok(())
}