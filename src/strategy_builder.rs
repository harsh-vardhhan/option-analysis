use crate::model::OptionData;
use crate::strategy::{Position, OptionType, Strategy};
use std::cmp::Ordering;

// Constants for Strategy Building
const STRIKE_WIDTH_NARROW: f64 = 100.0;
const STRIKE_WIDTH_WIDE: f64 = 200.0;
const DELTA_ATM: f64 = 0.5;
const DELTA_OTM_30: f64 = 0.3;
const DELTA_OTM_16: f64 = 0.16;
const MAX_SPREAD: f64 = 20.0;

pub struct StrategyBuilder;

impl StrategyBuilder {
    /// Builds a strategy based on the enum and option data.
    pub fn build(strategy: Strategy, data: &[OptionData]) -> Result<(Vec<Position>, String), String> {
        match strategy {
            Strategy::CallCreditSpread => Self::build_vertical_spread(data, OptionType::Call, false, DELTA_OTM_30, STRIKE_WIDTH_NARROW, "CCS"),
            Strategy::PutCreditSpread => Self::build_vertical_spread(data, OptionType::Put, false, -DELTA_OTM_30, STRIKE_WIDTH_NARROW, "PCS"),
            Strategy::CallDebitSpread => Self::build_vertical_spread(data, OptionType::Call, true, DELTA_ATM, STRIKE_WIDTH_NARROW, "CDS"),
            Strategy::PutDebitSpread => Self::build_vertical_spread(data, OptionType::Put, true, -DELTA_ATM, STRIKE_WIDTH_NARROW, "PDS"),
            Strategy::LongCall => Self::build_single_option(data, OptionType::Call, true, DELTA_ATM, "Long Call"),
            Strategy::LongPut => Self::build_single_option(data, OptionType::Put, true, -DELTA_ATM, "Long Put"),
            Strategy::ShortCall => Self::build_single_option(data, OptionType::Call, false, DELTA_OTM_30, "Short Call"),
            Strategy::ShortPut => Self::build_single_option(data, OptionType::Put, false, -DELTA_OTM_30, "Short Put"),
            Strategy::ShortStraddle => Self::build_straddle(data, "Short Straddle"),
            Strategy::ShortStrangle => Self::build_strangle(data, DELTA_OTM_16, "Short Strangle"),
            Strategy::IronButterfly => Self::build_iron_butterfly(data, "Iron Fly"),
            Strategy::IronCondor => Self::build_iron_condor(data, "Iron Condor"),
        }
    }

    // --- Strategy Implementations ---

    fn build_vertical_spread(
        data: &[OptionData], 
        kind: OptionType, 
        is_debit: bool, 
        anchor_delta: f64, 
        width: f64, 
        label: &str
    ) -> Result<(Vec<Position>, String), String> {
        let anchor_idx = if kind == OptionType::Call {
            Self::find_call_by_delta(data, anchor_delta)
        } else {
            Self::find_put_by_delta(data, anchor_delta)
        };

        if let Some(idx) = anchor_idx {
            let anchor_item = &data[idx];
            let anchor_strike = anchor_item.strike_price;
            let anchor_price = Self::get_price(anchor_item, kind)?;

            // Determine target strike for the second leg
            
            let width_sign = match kind {
                OptionType::Call => 1.0, 
                OptionType::Put => -1.0,
            };
            
            let target_strike = anchor_strike + (width * width_sign);
            
            let leg2_item = data.iter().find(|d| (d.strike_price - target_strike).abs() < 1.0);
            
            if let Some(leg2) = leg2_item {
                let leg2_price = Self::get_price(leg2, kind)?;
                
                let (qty1, qty2) = if is_debit { (1, -1) } else { (-1, 1) };
                
                let p1 = Position { strike: anchor_strike, kind, qty: qty1, entry_price: anchor_price };
                let p2 = Position { strike: target_strike, kind, qty: qty2, entry_price: leg2_price };
                
                Ok((vec![p1, p2], format!("Applied: {} ({}/{})", label, anchor_strike, target_strike)))
            } else {
                Err(format!("Error: Wing strike {} not found", target_strike))
            }
        } else {
            Err(format!("Error: Anchor leg (Delta ~{}) not found", anchor_delta))
        }
    }

    fn build_single_option(
        data: &[OptionData], 
        kind: OptionType, 
        is_buy: bool, 
        delta: f64, 
        label: &str
    ) -> Result<(Vec<Position>, String), String> {
        let idx_opt = if kind == OptionType::Call {
            Self::find_call_by_delta(data, delta)
        } else {
            Self::find_put_by_delta(data, delta)
        };

        if let Some(idx) = idx_opt {
            let item = &data[idx];
            let price = Self::get_price(item, kind)?;
            let qty = if is_buy { 1 } else { -1 };
            
            let pos = Position { strike: item.strike_price, kind, qty, entry_price: price };
            Ok((vec![pos], format!("Applied: {} ({})", label, item.strike_price)))
        } else {
            Err(format!("Error: Option (Delta ~{}) not found", delta))
        }
    }

    fn build_straddle(data: &[OptionData], label: &str) -> Result<(Vec<Position>, String), String> {
        // ATM Call & Put
        let c_idx = Self::find_call_by_delta(data, DELTA_ATM); 
        
        if let Some(idx) = c_idx {
            let item = &data[idx];
            let strike = item.strike_price;
            let call_price = Self::get_price(item, OptionType::Call)?;
            let put_price = Self::get_price(item, OptionType::Put)?; 
            
            // Safer:
            if item.put_options.is_none() {
                return Err(format!("Error: Put data missing for ATM strike {}", strike));
            }

            let p1 = Position { strike, kind: OptionType::Call, qty: -1, entry_price: call_price };
            let p2 = Position { strike, kind: OptionType::Put, qty: -1, entry_price: put_price };
            
            Ok((vec![p1, p2], format!("Applied: {} ({})", label, strike)))
        } else {
            Err("Error: ATM Strike not found".to_string())
        }
    }

    fn build_strangle(data: &[OptionData], delta: f64, label: &str) -> Result<(Vec<Position>, String), String> {
        let c_idx = Self::find_call_by_delta(data, delta);
        let p_idx = Self::find_put_by_delta(data, -delta);

        if let (Some(ci), Some(pi)) = (c_idx, p_idx) {
            let c_item = &data[ci];
            let p_item = &data[pi];
            
            let c_price = Self::get_price(c_item, OptionType::Call)?;
            let p_price = Self::get_price(p_item, OptionType::Put)?;

            let p1 = Position { strike: c_item.strike_price, kind: OptionType::Call, qty: -1, entry_price: c_price };
            let p2 = Position { strike: p_item.strike_price, kind: OptionType::Put, qty: -1, entry_price: p_price };
            
            Ok((vec![p1, p2], format!("Applied: {} ({}/{})", label, p_item.strike_price, c_item.strike_price)))
        } else {
            Err(format!("Error: Strangle legs (Delta ~{}) not found", delta))
        }
    }

    fn build_iron_butterfly(data: &[OptionData], label: &str) -> Result<(Vec<Position>, String), String> {
        let center_idx = Self::find_call_by_delta(data, DELTA_ATM);
        if let Some(idx) = center_idx {
            let center_item = &data[idx];
            let center_strike = center_item.strike_price;
            
            let u_strike = center_strike + STRIKE_WIDTH_WIDE;
            let l_strike = center_strike - STRIKE_WIDTH_WIDE;
            
            let u_item = data.iter().find(|d| (d.strike_price - u_strike).abs() < 1.0);
            let l_item = data.iter().find(|d| (d.strike_price - l_strike).abs() < 1.0);
            
            if let (Some(u), Some(l)) = (u_item, l_item) {
                let c_call = Self::get_price(center_item, OptionType::Call)?;
                let c_put = Self::get_price(center_item, OptionType::Put)?;
                let u_call = Self::get_price(u, OptionType::Call)?;
                let l_put = Self::get_price(l, OptionType::Put)?;
                
                let mut pos = Vec::new();
                pos.push(Position { strike: center_strike, kind: OptionType::Call, qty: -1, entry_price: c_call });
                pos.push(Position { strike: center_strike, kind: OptionType::Put, qty: -1, entry_price: c_put });
                pos.push(Position { strike: u_strike, kind: OptionType::Call, qty: 1, entry_price: u_call });
                pos.push(Position { strike: l_strike, kind: OptionType::Put, qty: 1, entry_price: l_put });
                
                Ok((pos, format!("Applied: {} ({})", label, center_strike)))
            } else {
                Err("Error: Wings for Iron Butterfly not found".to_string())
            }
        } else {
            Err("Error: ATM Center not found".to_string())
        }
    }

    fn build_iron_condor(data: &[OptionData], label: &str) -> Result<(Vec<Position>, String), String> {
        let c_idx = Self::find_call_by_delta(data, DELTA_OTM_16);
        let p_idx = Self::find_put_by_delta(data, -DELTA_OTM_16);
        
        if let (Some(ci), Some(pi)) = (c_idx, p_idx) {
            let sc_item = &data[ci];
            let sp_item = &data[pi];
            
            let sc_strike = sc_item.strike_price;
            let sp_strike = sp_item.strike_price;
            
            let lc_strike = sc_strike + STRIKE_WIDTH_WIDE;
            let lp_strike = sp_strike - STRIKE_WIDTH_WIDE;
            
            let lc_item = data.iter().find(|d| (d.strike_price - lc_strike).abs() < 1.0);
            let lp_item = data.iter().find(|d| (d.strike_price - lp_strike).abs() < 1.0);
            
            if let (Some(lc), Some(lp)) = (lc_item, lp_item) {
                let sc_p = Self::get_price(sc_item, OptionType::Call)?;
                let sp_p = Self::get_price(sp_item, OptionType::Put)?;
                let lc_p = Self::get_price(lc, OptionType::Call)?;
                let lp_p = Self::get_price(lp, OptionType::Put)?;

                let mut pos = Vec::new();
                pos.push(Position { strike: sc_strike, kind: OptionType::Call, qty: -1, entry_price: sc_p });
                pos.push(Position { strike: sp_strike, kind: OptionType::Put, qty: -1, entry_price: sp_p });
                pos.push(Position { strike: lc_strike, kind: OptionType::Call, qty: 1, entry_price: lc_p });
                pos.push(Position { strike: lp_strike, kind: OptionType::Put, qty: 1, entry_price: lp_p });
                
                Ok((pos, format!("Applied: {} ({}/{})", label, sp_strike, sc_strike)))
            } else {
                Err("Error: Wings for Iron Condor not found".to_string())
            }
        } else {
            Err("Error: Short legs (Delta ~0.16) not found".to_string())
        }
    }

    // --- Helpers ---

    fn get_price(data: &OptionData, kind: OptionType) -> Result<f64, String> {
        match kind {
            OptionType::Call => data.call_options.as_ref()
                .map(|o| o.market_data.ltp)
                .ok_or_else(|| format!("Call data missing for strike {}", data.strike_price)),
            OptionType::Put => data.put_options.as_ref()
                .map(|o| o.market_data.ltp)
                .ok_or_else(|| format!("Put data missing for strike {}", data.strike_price)),
        }
    }

    fn find_call_by_delta(data: &[OptionData], target: f64) -> Option<usize> {
        Self::find_with_filter(data, target, OptionType::Call, |spread, delta| {
            // Basic quality filter
             spread <= MAX_SPREAD && delta > 0.05 && delta < 0.95
        })
    }

    fn find_put_by_delta(data: &[OptionData], target: f64) -> Option<usize> {
         Self::find_with_filter(data, target, OptionType::Put, |spread, delta| {
             spread <= MAX_SPREAD && delta > -0.95 && delta < -0.05
         })
    }

    fn find_with_filter<F>(data: &[OptionData], target: f64, kind: OptionType, filter: F) -> Option<usize>
    where F: Fn(f64, f64) -> bool 
    {
        data.iter().enumerate()
            .filter(|(_, d)| {
                match kind {
                    OptionType::Call => {
                         if let Some(opt) = &d.call_options {
                            let spread = (opt.market_data.ask_price - opt.market_data.bid_price).abs();
                            let delta = opt.option_greeks.as_ref().map(|g| g.delta).unwrap_or(0.0);
                            filter(spread, delta)
                        } else { false }
                    },
                    OptionType::Put => {
                        if let Some(opt) = &d.put_options {
                            let spread = (opt.market_data.ask_price - opt.market_data.bid_price).abs();
                            let delta = opt.option_greeks.as_ref().map(|g| g.delta).unwrap_or(0.0);
                            filter(spread, delta)
                        } else { false }
                    }
                }
            })
            .min_by(|(_, a), (_, b)| {
                let get_delta = |item: &OptionData| -> f64 {
                    match kind {
                        OptionType::Call => item.call_options.as_ref().and_then(|o| o.option_greeks.as_ref()).map(|g| g.delta).unwrap_or(0.0),
                        OptionType::Put => item.put_options.as_ref().and_then(|o| o.option_greeks.as_ref()).map(|g| g.delta).unwrap_or(0.0),
                    }
                };
                
                let da = get_delta(a);
                let db = get_delta(b);
                (da - target).abs().partial_cmp(&(db - target).abs()).unwrap_or(Ordering::Equal)
            })
            .map(|(i, _)| i)
    }
}
