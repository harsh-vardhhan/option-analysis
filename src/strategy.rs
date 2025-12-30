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
}

impl Position {
    pub fn payoff(&self, spot_at_expiry: f64) -> f64 {
        let intrinsic = match self.kind {
            OptionType::Call => (spot_at_expiry - self.strike).max(0.0),
            OptionType::Put => (self.strike - spot_at_expiry).max(0.0),
        };
        
        let _pnl_per_qty = intrinsic - self.entry_price;
        // If Short (qty < 0): We received entry_price. PnL = (Entry - Intrinsic) * |Qty| = -(Intrinsic - Entry) * |Qty| = PnL_per_qty * Qty
        // Wait, standard convention:
        // Long Call: Pay 10. Spot 110 (Strike 100). Intrinsic 10. Net 0.
        // Pnl = (Intrinsic - Entry) * Qty
        
        // Short Call: Receive 10. Spot 110. Intrinsic 10. Net 0.
        // Pnl = (Intrinsic - Entry) * Qty = (10 - 10) * -1 = 0. Correct.
        // Short Call: Receive 10. Spot 120. Intrinsic 20. Net -10.
        // Pnl = (20 - 10) * -1 = -10. Correct.
        
        // Short Call: Receive 10. Spot 90. Intrinsic 0. Net +10.
        // Pnl = (0 - 10) * -1 = +10. Correct.
        
        // NIFTY 50 Lot Size
        let lot_size = 65.0; 
        (intrinsic - self.entry_price) * self.qty as f64 * lot_size
    }
}

pub fn calculate_net_payoff(positions: &[Position], spot: f64) -> f64 {
    positions.iter().map(|p| p.payoff(spot)).sum()
}

pub fn analyze_strategy(positions: &[Position], current_spot: f64) -> StrategyStats {
    if positions.is_empty() {
        return StrategyStats::default();
    }

    // Graph Range (for visualization)
    let lower_bound = current_spot * 0.85;
    let upper_bound = current_spot * 1.15;
    let steps = 100;
    let step_size = (upper_bound - lower_bound) / steps as f64;

    let mut max_profit = f64::NEG_INFINITY;
    let mut max_loss = f64::INFINITY; 
    let mut breakevens = Vec::new();
    let mut points = Vec::new();

    let mut prev_pnl = calculate_net_payoff(positions, lower_bound);
    
    // 1. Calculate Graph Points & Range Extrema
    for i in 0..=steps {
        let spot = lower_bound + i as f64 * step_size;
        let pnl = calculate_net_payoff(positions, spot);

        if pnl > max_profit { max_profit = pnl; }
        if pnl < max_loss { max_loss = pnl; }

        // Breakeven crossing
        if (prev_pnl < 0.0 && pnl >= 0.0) || (prev_pnl > 0.0 && pnl <= 0.0) {
            let prev_spot = spot - step_size;
            if (pnl - prev_pnl).abs() > 1e-6 {
                let zero_spot = prev_spot + (0.0 - prev_pnl) * (spot - prev_spot) / (pnl - prev_pnl);
                breakevens.push(zero_spot);
            } else {
                breakevens.push(spot);
            }
        }
        
        points.push((spot, pnl));
        prev_pnl = pnl;
    }

    // 2. Check Theoretical Limits (Unlimited P/L)
    // Test Extremes: 0 and a very high number (e.g., 5x Spot)
    let pnl_zero = calculate_net_payoff(positions, 0.0);
    // Determine slope near zero: PnL(0) vs PnL(1.0)
    let pnl_one = calculate_net_payoff(positions, 1.0);
    let _slope_low = pnl_one - pnl_zero;

    let high_spot = current_spot.max(100.0) * 10.0;
    let pnl_high = calculate_net_payoff(positions, high_spot);
    let pnl_high_minus = calculate_net_payoff(positions, high_spot - 1.0);
    let slope_high = pnl_high - pnl_high_minus;

    // Check Downside (Price -> 0)
    // If slope_low > 0 (Profit increases as price goes up from 0), then checks at 0 are:
    // PnL(0). If PnL(0) is massively positive (unlikely for options unless crazy arb) -> bounded by 0.
    // Real check: As Price goes to 0... 
    //    If Put Long: Payoff increases. Slope_low (dPnL/dSpot) is negative.
    //    If Put Short: Payoff decreases (Loss increases). Slope_low is positive (Loss lessens as price up).
    
    // Simpler check: Compare PnY at extremes
    // Ideally, for options, linear payof at extremes.
    // If slope_high > 0.01 -> Profit Unlimited Upside.
    // If slope_high < -0.01 -> Loss Unlimited Upside.
    
    // If slope_low < -0.01 -> Profit Unlimited Downside (Profit increases as Price drops).
    // If slope_low > 0.01 -> Loss Unlimited Downside (Loss increases as Price drops).
    
    let mut max_profit_unlimited = false;
    let mut max_loss_unlimited = false;

    // Upside
    if slope_high > 1e-4 { max_profit_unlimited = true; }
    if slope_high < -1e-4 { max_loss_unlimited = true; }

    // Downside (As Price -> 0)
    // Note: Price is bounded by 0, so Downside can technically be finite (Max Loss = Strike * Qty), but usually considered "Unlimited" or "Undefined Risk" in trader terms if it keeps growing till 0.
    // However, usually "Unlimited" implies infinite. 0 is finite. 
    // But for "Max Loss", Short Put is often called "undefined" or "unlimited" loosely, but strictly it is bounded by strike.
    // Let's stick to "Unlimited" meaning "Infinite". 
    // Short Put max loss is finite (Strike * Lot). 
    // Short Call max loss is infinite.
    
    // So distinct:
    // Slope High checks infinity.
    // Slope Low checks near 0.
    
    // Wait, let's stick to strict definitions.
    // Short Call: Loss grows as price -> inf. (slope_high < 0) -> Max Loss Unlimited.
    // Long Call: Profit grows as price -> inf. (slope_high > 0) -> Max Profit Unlimited.
    
    // Short Put: Loss grows as price -> 0. Finite max loss at 0. NOT Unlimited.
    // Long Put: Profit grows as price -> 0. Finite max profit at 0. NOT Unlimited.
    
    // So ONLY Upside produces true Unlimited PnL for standard options.
    
    // However, UI wise, if current Max Profit variable is holding the max of the *visible range*, we should update it to be the theoretical max if possible.
    // If bounded (Put side), the max is at 0 or high.
    // Let's update `max_profit` and `max_loss` to be the TRUE global max/min if they are finite.
    // If infinite, flag them.

    // Re-eval max/min including 0 and Low/High bounds
    let candidates = vec![pnl_zero, pnl_high];
    for val in candidates {
        if !max_profit_unlimited && val > max_profit { max_profit = val; }
        if !max_loss_unlimited && val < max_loss { max_loss = val; }
    }

    StrategyStats {
        max_profit,
        max_loss,
        max_profit_unlimited,
        max_loss_unlimited,
        breakevens,
        points,
    }
}
