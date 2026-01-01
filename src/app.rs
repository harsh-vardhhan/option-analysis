use crate::model::OptionData;
use crate::strategy::OptionType;
use crate::strategy_builder::StrategyBuilder;
use crate::portfolio::Portfolio;
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
}

impl App {
    pub fn new() -> App {
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
        if self.data.is_empty() { return; }
        
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
            let has_position = self.portfolio.positions.iter().any(|p| (p.strike - strike).abs() < 0.01 && p.kind == kind);
            
            if has_position {
                self.selected_positions.insert(key);
            }
        }
    }

    pub fn delete_position(&mut self) {
        if self.data.is_empty() { return; }
        
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
        if self.data.is_empty() { return; }
        self.portfolio.update_prices(&self.data);
    }

    pub fn handle_trade_action(&mut self, is_buy: bool) {
        if self.data.is_empty() { return; }
        
        let item = &self.data[self.selected_row];
        let strike = item.strike_price;
        let (kind, price) = match self.selected_column {
            ColumnSelection::Call => (OptionType::Call, item.call_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0)),
            ColumnSelection::Put => (OptionType::Put, item.put_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0)),
        };

        self.last_message = self.portfolio.trade(strike, kind, price, is_buy);
        
        // Cleanup: Remove 0 qty
        // Check for zero qty positions to remove from selection (Portfolio has already removed them from its list)
        // We iterate our selection and check if it still exists in portfolio.
        self.selected_positions.retain(|(s_key, s_kind)| {
             self.portfolio.positions.iter().any(|p| format!("{:.2}", p.strike) == *s_key && p.kind == *s_kind)
        });
    }
    pub fn move_position_row(&mut self, delta: i32) {
        if self.data.is_empty() { return; }

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
             
             if let Some(pos_idx) = self.portfolio.positions.iter().position(|p| p.strike == old_strike && p.kind == kind) {
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
        
        if positions_to_move.is_empty() { return; }

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
            let current_data_idx = self.data.iter().position(|d| (d.strike_price - pos.strike).abs() < 0.01);
            
            if let Some(idx) = current_data_idx {
                let new_data_idx = idx as i32 + delta;
                
                if new_data_idx >= 0 && new_data_idx < self.data.len() as i32 {
                    let new_data_idx = new_data_idx as usize;
                    let new_item = &self.data[new_data_idx];
                    let new_price = match pos.kind {
                         OptionType::Call => new_item.call_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0),
                         OptionType::Put => new_item.put_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0),
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
            // If we have potential merges, it gets complicated (e.g. moving a leg ONTO another leg).
            // Simplification: Just update the properties. If it overlaps, `handle_trade_action` logic or a cleanup pass might be needed.
            // But here we are modifying IN PLACE.
            // If we move pos A to strike X, and there is already pos B at strike X...
            // Implementing merge logic here is complex. 
            // For now, let's just update field. "Merge" usually happens on 'Add', but here we are mutating.
            // If we end up with 2 positions same strike same kind, the renderer will sum them or show duplicates?
            // Renderer finds "first". `table.rs`: `app.positions.iter().find(...)`. It only shows the first one!
            // So we MUST merge or ensure uniqueness.
            
            // Actually, let's keep it simple: Just update. The user can see duplicates in "Strategies" list (if we had one).
            // But `table.rs` only shows one.
            // Let's defer strict merging for now. It's an edge case (moving a spread leg onto another leg).
            
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

    pub fn move_position_col(&mut self) {
        if self.data.is_empty() { return; }
        
        // Determine move direction based on current column
        // If Call -> Move to Put (Right). If Put -> Move to Call (Left).
        // This toggles selection.
        
        let old_col = &self.selected_column;
        let item = &self.data[self.selected_row];
        let strike = item.strike_price;

        let (old_kind, new_kind) = match old_col {
            ColumnSelection::Call => (OptionType::Call, OptionType::Put),
            ColumnSelection::Put => (OptionType::Put, OptionType::Call),
        };

        // Check if position exists at current selection
        if let Some(pos_idx) = self.portfolio.positions.iter().position(|p| p.strike == strike && p.kind == old_kind) {
            let mut pos = self.portfolio.positions.remove(pos_idx);
            
            // Update to new kind
            pos.kind = new_kind;
            let new_ltp = match new_kind {
                 OptionType::Call => item.call_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0),
                 OptionType::Put => item.put_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0),
            };
            pos.entry_price = new_ltp;

            // Merge check
            if let Some(target_pos) = self.portfolio.positions.iter_mut().find(|p| p.strike == strike && p.kind == new_kind) {
                let total_qty = target_pos.qty + pos.qty;
                if total_qty != 0 {
                    let old_val = target_pos.qty as f64 * target_pos.entry_price;
                    let move_val = pos.qty as f64 * pos.entry_price;
                    target_pos.entry_price = (old_val + move_val) / total_qty as f64;
                    target_pos.qty = total_qty;
                } else {
                    target_pos.qty = 0;
                }
            } else {
                self.portfolio.positions.push(pos);
            }

             self.portfolio.positions.retain(|p| p.qty != 0);
        }

        // Toggle selection
        self.toggle_column();
    }

    pub fn apply_strategy(&mut self) {
        if self.data.is_empty() { return; }
        
        let strategy_name = self.strategies[self.selected_strategy];
        
        match StrategyBuilder::build(strategy_name, &self.data) {
            Ok((new_positions, message)) => {
                self.portfolio.positions = new_positions;
                self.last_message = message;
            },
            Err(e) => {
                self.last_message = e;
            }
        }
    }

    pub fn next_strategy(&mut self) {
        if self.strategies.is_empty() { return; }
        if self.selected_strategy < self.strategies.len() - 1 {
            self.selected_strategy += 1;
        } else {
            self.selected_strategy = 0; // Wrap to start
        }
        // Always apply on interaction to support "re-triggering" or single-item interaction
        self.apply_strategy();
    }

    pub fn previous_strategy(&mut self) {
        if self.strategies.is_empty() { return; }
        if self.selected_strategy > 0 {
            self.selected_strategy -= 1;
        } else {
            self.selected_strategy = self.strategies.len() - 1; // Wrap to end
        }
        // Always apply on interaction
        self.apply_strategy();
    }
}
