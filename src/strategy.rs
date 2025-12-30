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

    // Scan range: +/- 10% of spot (Usually enough for graph)
    // Actually strategy builder graphs usually center on the action. 
    // If straddle at 20000, spot 22000, we want to see 20000.
    // Let's use simple heuristic: +/- 15% of spot for now.
    let lower_bound = current_spot * 0.85;
    let upper_bound = current_spot * 1.15;
    let steps = 100;
    let step_size = (upper_bound - lower_bound) / steps as f64;

    let mut max_profit = f64::NEG_INFINITY;
    let mut max_loss = f64::INFINITY; // Loss is negative profit, so we store min_pnl
    let mut breakevens = Vec::new();
    let mut points = Vec::new();

    let mut prev_pnl = calculate_net_payoff(positions, lower_bound);
    
    // Check range
    for i in 0..=steps {
        let spot = lower_bound + i as f64 * step_size;
        let pnl = calculate_net_payoff(positions, spot);

        if pnl > max_profit { max_profit = pnl; }
        if pnl < max_loss { max_loss = pnl; }

        // Breakeven crossing
        if (prev_pnl < 0.0 && pnl >= 0.0) || (prev_pnl > 0.0 && pnl <= 0.0) {
            // Linear interpolation using y = mx + c logic for better precision on BE point
            // spot = x, pnl = y. y0 at x0 (prev_spot), y1 at x1 (spot)
            // x_zero = x0 + (0 - y0) * (x1 - x0) / (y1 - y0)
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

    // Heuristics for Infinite/Undefined
    // If slope at ends is non-zero, it's infinite.
    // Simplifying: Just report the scanned range extrema for now, user understands "Terminal" limitations.
    // Or we can cap it nicely.

    StrategyStats {
        max_profit,
        max_loss,
        breakevens,
        points,
    }
}
