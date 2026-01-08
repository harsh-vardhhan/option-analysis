use crate::strategy::{Position, OptionType};
use crate::model::OptionData;

pub struct Portfolio {
    pub positions: Vec<Position>,
}

impl Portfolio {
    pub fn new() -> Self {
        Portfolio {
            positions: Vec::new(),
        }
    }

    pub fn trade(&mut self, strike: f64, kind: OptionType, price: f64, is_buy: bool) -> String {
        let side = if is_buy { "BUY" } else { "SELL" };
        let k_str = match kind {
            OptionType::Call => "CE",
            OptionType::Put => "PE",
        };

        if let Some(pos) = self.positions.iter_mut().find(|p| (p.strike - strike).abs() < 0.01 && p.kind == kind) {
             // Flip logic or increment
             if is_buy {
                if pos.qty < 0 {
                     pos.qty = 1;
                     pos.entry_price = price;
                } else {
                    pos.qty += 1;
                    let old_total = pos.entry_price * (pos.qty - 1) as f64;
                    pos.entry_price = (old_total + price) / pos.qty as f64;
                }
            } else if pos.qty > 0 {
                pos.qty = -1;
                pos.entry_price = price;
            } else {
                pos.qty -= 1;
                let old_qty_abs = (pos.qty + 1).abs() as f64;
                let old_total = pos.entry_price * old_qty_abs;
                pos.entry_price = (old_total + price) / pos.qty.abs() as f64;
            }
        } else {
            // New Position
            self.positions.push(Position {
                strike,
                kind,
                qty: if is_buy { 1 } else { -1 },
                entry_price: price,
            });
        }

        // Cleanup: Remove 0 qty
        self.positions.retain(|p| p.qty != 0);

        format!("{} {} {} @ {:.2}", side, k_str, strike, price)
    }

    pub fn remove(&mut self, strike: f64, kind: OptionType) {
        self.positions.retain(|p| !((p.strike - strike).abs() < 0.01 && p.kind == kind));
    }

    pub fn update_prices(&mut self, data: &[OptionData]) {
        for pos in &mut self.positions {
            // Find current market price for this position
            if let Some(market_row) = data.iter().find(|d| (d.strike_price - pos.strike).abs() < 0.1) {
                let current_ltp = match pos.kind {
                    OptionType::Call => market_row.call_options.as_ref().map(|o| o.market_data.ltp),
                    OptionType::Put => market_row.put_options.as_ref().map(|o| o.market_data.ltp),
                };

                if let Some(price) = current_ltp {
                    pos.entry_price = price;
                }
            }
        }
    }
}


