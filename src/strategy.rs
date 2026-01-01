use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum OptionType {
    Call,
    Put,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub strike: f64,
    pub kind: OptionType,
    pub qty: i32, // + for Buy, - for Sell
    pub entry_price: f64,
}

#[derive(Debug, Default)]
pub struct StrategyStats {
    pub max_profit: f64,
    pub max_loss: f64,
    pub max_profit_unlimited: bool,
    pub max_loss_unlimited: bool,
    pub breakevens: Vec<f64>,
    pub points: Vec<(f64, f64)>,
    pub pop: f64, // Probability of Profit [0.0, 1.0]
}

impl Position {
    pub fn payoff(&self, spot_at_expiry: f64) -> f64 {
        let intrinsic = match self.kind {
            OptionType::Call => (spot_at_expiry - self.strike).max(0.0),
            OptionType::Put => (self.strike - spot_at_expiry).max(0.0),
        };
        
        let lot_size = 65.0; 
        // Note: entry_price is per unit.
        // If Buy: Pay entry_price. PnL = (Intrinsic - Entry)
        // If Sell: Receive entry_price. PnL = (Entry - Intrinsic) = -(Intrinsic - Entry)
        // But qty handles the sign (-1 for Sell).
        // So (Intrinsic - Entry) * Qty is correct.
        (intrinsic - self.entry_price) * self.qty as f64 * lot_size
    }
}

pub fn calculate_net_payoff(positions: &[Position], spot: f64) -> f64 {
    positions.iter().map(|p| p.payoff(spot)).sum()
}
// A more standard approximation for CDF (Abramowitz and Stegun 26.2.17)
fn std_normal_cdf(x: f64) -> f64 {
    let b1 = 0.319381530;
    let b2 = -0.356563782;
    let b3 = 1.781477937;
    let b4 = -1.821255978;
    let b5 = 1.330274429;
    let p = 0.2316419;
    let c2 = 0.39894228;

    let abs_x = x.abs();
    let t = 1.0 / (1.0 + p * abs_x);
    let val = 1.0 - c2 * (-x * x / 2.0).exp() *
        (t * (b1 + t * (b2 + t * (b3 + t * (b4 + t * b5)))));

    if x < 0.0 {
        1.0 - val
    } else {
        val
    }
}


pub fn analyze_strategy(
    positions: &[Position], 
    current_spot: f64, 
    iv: f64, 
    days_to_expiry: f64
) -> StrategyStats {
    if positions.is_empty() {
        return StrategyStats::default();
    }

    // Graph Range (Standard Deviation based now?)
    // 2 SD move for viz is good.
    let volatility_range = if iv > 0.0 {
        current_spot * (iv / 100.0) * (days_to_expiry / 365.0).sqrt() * 3.0 // 3 SD
    } else {
        current_spot * 0.05
    };
    
    let range_min = current_spot - volatility_range;
    let range_max = current_spot + volatility_range;
    
    // Extend for strikes
    let strikes: Vec<f64> = positions.iter().map(|p| p.strike).collect();
    let min_strike = strikes.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_strike = strikes.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    
    let lower_bound = range_min.min(min_strike * 0.98);
    let upper_bound = range_max.max(max_strike * 1.02);

    let steps = 200;
    let step_size = (upper_bound - lower_bound) / steps as f64;

    let mut max_profit = f64::NEG_INFINITY;
    let mut max_loss = f64::INFINITY; 
    let mut breakevens = Vec::new();
    let mut points = Vec::new();

    let mut prev_pnl = calculate_net_payoff(positions, lower_bound);
    
    // 1. Calculate Graph Points & Range Extrema & Breakevens
    for i in 0..=steps {
        let spot = lower_bound + i as f64 * step_size;
        let pnl = calculate_net_payoff(positions, spot);

        if pnl > max_profit { max_profit = pnl; }
        if pnl < max_loss { max_loss = pnl; }

        // Breakeven crossing
        if (prev_pnl < 0.0 && pnl >= 0.0) || (prev_pnl > 0.0 && pnl <= 0.0) {
            let prev_spot = spot - step_size;
            // Linear iterpolation for precise root
            if (pnl - prev_pnl).abs() > 1e-9 {
                let zero_spot = prev_spot + (0.0 - prev_pnl) * (spot - prev_spot) / (pnl - prev_pnl);
                breakevens.push(zero_spot);
            } else {
                breakevens.push(spot);
            }
        }
        
        points.push((spot, pnl));
        prev_pnl = pnl;
    }

    // 2. Value at Exact Spot (if not covered) - Points cover it roughly.

    // 3. Unlimited Checks (Slope at ends)
    let pnl_high = calculate_net_payoff(positions, current_spot * 10.0);
    let pnl_high_minus = calculate_net_payoff(positions, current_spot * 10.0 - 1.0);
    let slope_high = pnl_high - pnl_high_minus;

    let mut max_profit_unlimited = false;
    let mut max_loss_unlimited = false;

    if slope_high > 1e-4 { max_profit_unlimited = true; }
    if slope_high < -1e-4 { max_loss_unlimited = true; }
    
    let pnl_zero = calculate_net_payoff(positions, 0.0);
    
    let candidates = vec![pnl_zero, pnl_high];
    for val in candidates {
        if !max_profit_unlimited && val > max_profit { max_profit = val; }
        if !max_loss_unlimited && val < max_loss { max_loss = val; }
    }

    // 4. Calculate PoP
    let mut pop = 0.0;
    if iv > 0.0 && days_to_expiry > 0.0 {
        let t_years = days_to_expiry / 365.0;
        let sigma = iv / 100.0;
        let sigma_sqrt_t = sigma * t_years.sqrt();
        let drift = -0.5 * sigma * sigma * t_years; // assuming r=0
        
        // Helper to get prob < X (Prob Price ends below X)
        // ln(ST/S0) ~ N(drift, vol)
        // ln(ST) - ln(S0) < Z
        // ln(X/S0) < Z
        let prob_below = |price: f64| -> f64 {
            if price <= 0.0 { return 0.0; }
            let d2 = ( (price / current_spot).ln() - drift ) / sigma_sqrt_t;
            std_normal_cdf(d2)
        };

        // We identify profitable intervals.
        // Breakevens divide the line into segments.
        // We test a point in each segment.
        
        let mut sorted_points = breakevens.clone();
        sorted_points.push(0.0); // Start
        sorted_points.push(f64::INFINITY); // End
        sorted_points.sort_by(|a, b| a.partial_cmp(b).unwrap());
        sorted_points.dedup();
        
        for window in sorted_points.windows(2) {
            let start = window[0];
            let end = window[1];
            
            // Test mid point
            let test_point = if end == f64::INFINITY {
                start + 1.0 // just above start
            } else if start == 0.0 {
                end * 0.5
            } else {
                (start + end) / 2.0
            };
            
            if calculate_net_payoff(positions, test_point) > 0.0 {
                // This segment is profitable. Add probability mass.
                let p_end = if end == f64::INFINITY { 1.0 } else { prob_below(end) };
                let p_start = prob_below(start);
                pop += p_end - p_start;
            }
        }
    }

    StrategyStats {
        max_profit,
        max_loss,
        max_profit_unlimited,
        max_loss_unlimited,
        breakevens,
        points,
        pop,
    }
}
