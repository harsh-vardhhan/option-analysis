use crate::model::OptionData;
use crate::portfolio::Portfolio;
use crate::strategy::OptionType;
use crate::strategy_builder::StrategyBuilder;
use ratatui::widgets::TableState;
use std::collections::HashSet;

pub enum ColumnSelection {
    Call,
    Put,
}

#[derive(PartialEq)]
pub enum Focus {
    OptionChain,
    Strategies,
}

pub struct App {
    pub data: Vec<OptionData>,
    pub selected_row: usize,
    pub selected_column: ColumnSelection,
    pub should_quit: bool,
    pub initial_centering_done: bool,
    pub portfolio: Portfolio,
    pub last_message: String,
    pub table_state: TableState,
    pub show_help: bool,

    // New Fields
    pub active_focus: Focus,
    pub strategies: Vec<crate::strategy::Strategy>,
    pub selected_strategy: usize,

    // Multi-select
    // Using String for strike to avoid float key issues: format!("{:.2}", strike)
    pub selected_positions: HashSet<(String, OptionType)>,

    // Expiry Management
    pub available_expiries: Vec<String>,
    pub current_expiry_index: usize,

    // Bid/Ask Depth
    pub market_depth: Option<crate::model::QuoteData>,
}

impl App {
    pub fn get_selected_instrument_key(&self) -> Option<String> {
        if self.data.is_empty() {
            return None;
        }
        if self.selected_row >= self.data.len() {
            return None;
        }

        let item = &self.data[self.selected_row];
        match self.selected_column {
            ColumnSelection::Call => item.call_options.as_ref().map(|o| o.instrument_key.clone()),
            ColumnSelection::Put => item.put_options.as_ref().map(|o| o.instrument_key.clone()),
        }
    }

    pub fn new() -> App {
        let all_expiries = vec![
            "06 Jan 2026",
            "13 Jan 2026",
            "20 Jan 2026",
            "27 Jan 2026",
            "03 Feb 2026",
            "24 Feb 2026",
            "30 Mar 2026",
            "30 Jun 2026",
            "29 Sep 2026",
            "29 Dec 2026",
        ];

        // Filter expiries: Keep only those >= today
        // Note: In a real app we'd use Local::now().date_naive(), but for this specific request
        // regarding the provided list and "2026", we'll just implement the logic.
        // The user prompt says "once the expiry date is behind the current date, don't show it anymore".
        let today = chrono::Local::now().date_naive();
        let available_expiries: Vec<String> = all_expiries
            .into_iter()
            .filter_map(|date_str| {
                // Parse date "06 Jan 2026". Format: "%d %b %Y"
                if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%d %b %Y") {
                    if date >= today {
                        return Some(date.format("%Y-%m-%d").to_string());
                    }
                }
                None
            })
            .collect();

        let initial_expiry_index = 0; // Default to first available (nearest future)

        App {
            data: Vec::new(),
            selected_row: 0,
            selected_column: ColumnSelection::Call,
            should_quit: false,
            initial_centering_done: false,
            portfolio: Portfolio::new(),
            last_message: String::from("Ready"),
            table_state: TableState::default(),
            show_help: false,

            active_focus: Focus::OptionChain,
            strategies: crate::strategy::Strategy::all().to_vec(),
            selected_strategy: 0,
            selected_positions: HashSet::new(),

            available_expiries,
            current_expiry_index: initial_expiry_index,
            market_depth: None,
        }
    }

    pub fn on_tick(&mut self) {
        // Handle tick logic if needed (e.g. data updates could be pushed here)
    }

    pub fn next_row(&mut self) {
        if !self.data.is_empty() && self.selected_row < self.data.len() - 1 {
            self.selected_row += 1;
        }
    }

    pub fn previous_row(&mut self) {
        if self.selected_row > 0 {
            self.selected_row -= 1;
        }
    }

    pub fn toggle_column(&mut self) {
        self.selected_column = match self.selected_column {
            ColumnSelection::Call => ColumnSelection::Put,
            ColumnSelection::Put => ColumnSelection::Call,
        };
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn toggle_selection(&mut self) {
        if self.data.is_empty() {
            return;
        }

        let item = &self.data[self.selected_row];
        let strike_key = format!("{:.2}", item.strike_price);
        let kind = match self.selected_column {
            ColumnSelection::Call => OptionType::Call,
            ColumnSelection::Put => OptionType::Put,
        };

        // Toggle
        let key = (strike_key, kind);
        if self.selected_positions.contains(&key) {
            self.selected_positions.remove(&key);
        } else {
            // Only allow selection if a position exists at this strike/kind
            let strike = item.strike_price;
            let has_position = self
                .portfolio
                .positions
                .iter()
                .any(|p| (p.strike - strike).abs() < 0.01 && p.kind == kind);

            if has_position {
                self.selected_positions.insert(key);
            }
        }
    }

    pub fn delete_position(&mut self) {
        if self.data.is_empty() {
            return;
        }

        let item = &self.data[self.selected_row];
        let strike = item.strike_price;
        let kind = match self.selected_column {
            ColumnSelection::Call => OptionType::Call,
            ColumnSelection::Put => OptionType::Put,
        };

        // Remove from selection if present
        let key = (format!("{:.2}", strike), kind);
        if self.selected_positions.contains(&key) {
            self.selected_positions.remove(&key);
        }

        self.portfolio.remove(strike, kind);
    }

    pub fn update_live_prices(&mut self) {
        if self.data.is_empty() {
            return;
        }
        self.portfolio.update_prices(&self.data);
    }

    pub fn handle_trade_action(&mut self, is_buy: bool) {
        if self.data.is_empty() {
            return;
        }

        let item = &self.data[self.selected_row];
        let strike = item.strike_price;
        let (kind, price) = match self.selected_column {
            ColumnSelection::Call => (
                OptionType::Call,
                item.call_options
                    .as_ref()
                    .map(|o| o.market_data.ltp)
                    .unwrap_or(0.0),
            ),
            ColumnSelection::Put => (
                OptionType::Put,
                item.put_options
                    .as_ref()
                    .map(|o| o.market_data.ltp)
                    .unwrap_or(0.0),
            ),
        };

        self.last_message = self.portfolio.trade(strike, kind, price, is_buy);

        // Cleanup: Remove 0 qty
        // Check for zero qty positions to remove from selection (Portfolio has already removed them from its list)
        // We iterate our selection and check if it still exists in portfolio.
        self.selected_positions.retain(|(s_key, s_kind)| {
            self.portfolio
                .positions
                .iter()
                .any(|p| format!("{:.2}", p.strike) == *s_key && p.kind == *s_kind)
        });
    }
    pub fn move_position_row(&mut self, delta: i32) {
        if self.data.is_empty() {
            return;
        }

        let current_row = self.selected_row;
        let target_row_idx = current_row as i32 + delta;

        // 1. Always move cursor if valid
        if target_row_idx >= 0 && target_row_idx < self.data.len() as i32 {
            self.selected_row = target_row_idx as usize;
        } else {
            // If cursor can't move, we probably shouldn't move positions either?
            // Or maybe we should? Let's restrict movement to valid data range.
            return;
        }

        // 2. Identify positions to move
        // If selection is empty, try to move the one under cursor ONLY if it exists.
        // If selection is NOT empty, move ALL selected positions.

        let mut positions_to_move: Vec<usize> = Vec::new(); // Indices in self.positions

        if self.selected_positions.is_empty() {
            // Fallback: Check cursor position
            let old_item = &self.data[current_row];
            let old_strike = old_item.strike_price;
            let kind = match self.selected_column {
                ColumnSelection::Call => OptionType::Call,
                ColumnSelection::Put => OptionType::Put,
            };

            if let Some(pos_idx) = self
                .portfolio
                .positions
                .iter()
                .position(|p| p.strike == old_strike && p.kind == kind)
            {
                positions_to_move.push(pos_idx);
            }
        } else {
            // Find all indices that match selected keys
            for (i, p) in self.portfolio.positions.iter().enumerate() {
                let key = (format!("{:.2}", p.strike), p.kind);
                if self.selected_positions.contains(&key) {
                    positions_to_move.push(i);
                }
            }
        }

        if positions_to_move.is_empty() {
            return;
        }

        // 3. Validate ALL moves first
        // We need to know "delta indices".
        // Problem: Strikes might not be linear indices if data is not sorted or has gaps (though usually option chain is sorted).
        // Assuming `self.data` is sorted by strike.
        // We find the current index of each position's strike in `self.data`, apply `delta`, and see if it lands on a valid index.

        let mut changes: Vec<(usize, f64, f64)> = Vec::new(); // (pos_index, new_strike, new_price)

        for &pos_idx in &positions_to_move {
            let pos = &self.portfolio.positions[pos_idx];

            // Find current data index for this position's strike
            // Optimization: Assuming sorted, could use binary search, but linear scan is fine for < 200 items.
            // Using a tolerance for float comparison
            let current_data_idx = self
                .data
                .iter()
                .position(|d| (d.strike_price - pos.strike).abs() < 0.01);

            if let Some(idx) = current_data_idx {
                let new_data_idx = idx as i32 + delta;

                if new_data_idx >= 0 && new_data_idx < self.data.len() as i32 {
                    let new_data_idx = new_data_idx as usize;
                    let new_item = &self.data[new_data_idx];
                    let new_price = match pos.kind {
                        OptionType::Call => new_item
                            .call_options
                            .as_ref()
                            .map(|o| o.market_data.ltp)
                            .unwrap_or(0.0),
                        OptionType::Put => new_item
                            .put_options
                            .as_ref()
                            .map(|o| o.market_data.ltp)
                            .unwrap_or(0.0),
                    };
                    changes.push((pos_idx, new_item.strike_price, new_price));
                } else {
                    // One of the legs would go out of bounds. Abort ALL movement?
                    // Typically yes, to maintain the spread structure.
                    return;
                }
            } else {
                // Position strike not found in data? Can't move it.
                return;
            }
        }

        // 4. Execute Moves
        // We update input positions.
        // We also need to update `selected_positions` keys if we are in multi-select mode.

        let mut temp_selection_updates: Vec<(String, OptionType, String)> = Vec::new(); // (old_key_strike, kind, new_key_strike)

        for (pos_idx, new_strike, new_price) in changes {
            // Update position details
            // Note: Merging logic simplified for now; duplicates accepted visually if they occur.

            let pos = &mut self.portfolio.positions[pos_idx];
            let old_strike_key = format!("{:.2}", pos.strike);
            let kind = pos.kind;

            pos.strike = new_strike;
            pos.entry_price = new_price;

            let new_strike_key = format!("{:.2}", new_strike);
            temp_selection_updates.push((old_strike_key, kind, new_strike_key));
        }

        // 5. Update Selection Keys
        if !self.selected_positions.is_empty() {
            for (old_s, kind, new_s) in temp_selection_updates {
                let old_key = (old_s, kind);
                if self.selected_positions.contains(&old_key) {
                    self.selected_positions.remove(&old_key);
                    self.selected_positions.insert((new_s, kind));
                }
            }
        }
    }

    pub fn apply_strategy(&mut self) {
        if self.data.is_empty() {
            return;
        }

        let strategy_name = self.strategies[self.selected_strategy];

        match StrategyBuilder::build(strategy_name, &self.data) {
            Ok((new_positions, message)) => {
                self.portfolio.positions = new_positions;
                self.last_message = message;
            }
            Err(e) => {
                self.last_message = e;
            }
        }
    }

    pub fn next_strategy(&mut self) {
        if self.strategies.is_empty() {
            return;
        }
        if self.selected_strategy < self.strategies.len() - 1 {
            self.selected_strategy += 1;
        } else {
            self.selected_strategy = 0; // Wrap to start
        }
        // Always apply on interaction to support "re-triggering" or single-item interaction
        self.apply_strategy();
    }

    pub fn previous_strategy(&mut self) {
        if self.strategies.is_empty() {
            return;
        }
        if self.selected_strategy > 0 {
            self.selected_strategy -= 1;
        } else {
            self.selected_strategy = self.strategies.len() - 1; // Wrap to end
        }
        // Always apply on interaction
        self.apply_strategy();
    }

    pub fn next_expiry(&mut self) -> bool {
        if self.available_expiries.is_empty() {
            return false;
        }
        if self.current_expiry_index < self.available_expiries.len() - 1 {
            self.current_expiry_index += 1;
            return true; // value changed
        }
        false
    }

    pub fn previous_expiry(&mut self) -> bool {
        if self.available_expiries.is_empty() {
            return false;
        }
        if self.current_expiry_index > 0 {
            self.current_expiry_index -= 1;
            return true; // value changed
        }
        false
    }

    pub fn calculate_strategy_stats(&self) -> crate::strategy::StrategyStats {
        use chrono::{Local, NaiveDate};

        // Helper to get spot price for Strategy Analysis
        let spot_price = self
            .data
            .first()
            .map(|d| d.underlying_spot_price)
            .unwrap_or(0.0);

        // Calculate Days to Expiry
        let expiry_str = self.data.first().map(|d| d.expiry.as_str()).unwrap_or("");
        let days_to_expiry =
            if let Ok(expiry_date) = NaiveDate::parse_from_str(expiry_str, "%Y-%m-%d") {
                let today = Local::now().date_naive();
                (expiry_date - today).num_days().max(1) as f64 // at least 1 day to avoid div/0
            } else {
                1.0
            };

        // Find ATM IV (Average of Call/Put IV at closest strike)
        let atm_iv = if !self.data.is_empty() {
            let closest = self.data.iter().min_by(|a, b| {
                (a.strike_price - spot_price)
                    .abs()
                    .partial_cmp(&(b.strike_price - spot_price).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            if let Some(row) = closest {
                let call_iv = row
                    .call_options
                    .as_ref()
                    .and_then(|o| o.option_greeks.as_ref())
                    .map(|g| g.iv)
                    .unwrap_or(0.0);
                let put_iv = row
                    .put_options
                    .as_ref()
                    .and_then(|o| o.option_greeks.as_ref())
                    .map(|g| g.iv)
                    .unwrap_or(0.0);
                if call_iv > 0.0 && put_iv > 0.0 {
                    (call_iv + put_iv) / 2.0
                } else if call_iv > 0.0 {
                    call_iv
                } else {
                    put_iv
                }
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Calculate Chain Step
        let chain_step = if self.data.len() > 1 {
            let mut strikes: Vec<f64> = self.data.iter().map(|d| d.strike_price).collect();
            strikes.sort_by(|a, b| a.total_cmp(b));
            strikes.dedup();

            let mut min_diff = f64::INFINITY;
            for window in strikes.windows(2) {
                let diff = (window[1] - window[0]).abs();
                if diff < min_diff && diff > 1.0 {
                    min_diff = diff;
                }
            }
            if min_diff != f64::INFINITY {
                min_diff
            } else {
                50.0
            }
        } else {
            50.0
        };

        use crate::strategy::analyze_strategy;
        analyze_strategy(
            &self.portfolio.positions,
            spot_price,
            atm_iv,
            days_to_expiry,
            chain_step,
        )
    }
}
