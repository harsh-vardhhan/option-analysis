use crate::model::OptionData;
use crate::strategy::{Position, OptionType};

pub enum ColumnSelection {
    Call,
    Put,
}

pub struct App {
    pub data: Vec<OptionData>,
    pub selected_row: usize,
    pub selected_column: ColumnSelection,
    pub should_quit: bool,
    pub initial_centering_done: bool,
    pub positions: Vec<Position>,
    pub last_message: String,
}

impl App {
    pub fn new() -> App {
        App {
            data: Vec::new(),
            selected_row: 0,
            selected_column: ColumnSelection::Call,
            should_quit: false,
            initial_centering_done: false,
            positions: Vec::new(),
            last_message: String::from("Ready"),
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

    pub fn delete_position(&mut self) {
        if self.data.is_empty() { return; }
        
        let item = &self.data[self.selected_row];
        let strike = item.strike_price;
        let kind = match self.selected_column {
            ColumnSelection::Call => OptionType::Call,
            ColumnSelection::Put => OptionType::Put,
        };

        self.positions.retain(|p| !(p.strike == strike && p.kind == kind));
    }

    pub fn update_live_prices(&mut self) {
        if self.data.is_empty() { return; }

        for pos in &mut self.positions {
            // Find current market price for this position
            if let Some(market_row) = self.data.iter().find(|d| (d.strike_price - pos.strike).abs() < 0.1) {
                let current_ltp = match pos.kind {
                    crate::strategy::OptionType::Call => market_row.call_options.as_ref().map(|o| o.market_data.ltp),
                    crate::strategy::OptionType::Put => market_row.put_options.as_ref().map(|o| o.market_data.ltp),
                };

                if let Some(price) = current_ltp {
                    pos.entry_price = price;
                }
            }
        }
    }

    pub fn handle_trade_action(&mut self, is_buy: bool) {
        if self.data.is_empty() { return; }
        
        let item = &self.data[self.selected_row];
        let strike = item.strike_price;
        let (kind, price) = match self.selected_column {
            ColumnSelection::Call => (OptionType::Call, item.call_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0)),
            ColumnSelection::Put => (OptionType::Put, item.put_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0)),
        };

        if let Some(pos) = self.positions.iter_mut().find(|p| p.strike == strike && p.kind == kind) {
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
            } else {
                if pos.qty > 0 {
                    pos.qty = -1;
                    pos.entry_price = price;
                } else {
                    pos.qty -= 1;
                    let old_qty_abs = (pos.qty + 1).abs() as f64;
                    let old_total = pos.entry_price * old_qty_abs;
                    pos.entry_price = (old_total + price) / pos.qty.abs() as f64;
                }
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
        
        // Cleanup: Remove 0 qty?
        self.positions.retain(|p| p.qty != 0);

        // Update Message
        let side = if is_buy { "BUY" } else { "SELL" };
        let k_str = match kind {
            OptionType::Call => "CE",
            OptionType::Put => "PE",
        };
        self.last_message = format!("{} {} {} @ {:.2}", side, k_str, strike, price);
    }
    pub fn move_position_row(&mut self, delta: i32) {
        if self.data.is_empty() { return; }

        let current_idx = self.selected_row;
        let new_idx = current_idx as i32 + delta;

        if new_idx < 0 || new_idx >= self.data.len() as i32 {
            return;
        }

        let new_idx = new_idx as usize;
        
        let old_item = &self.data[current_idx];
        let old_strike = old_item.strike_price;
        let kind = match self.selected_column {
            ColumnSelection::Call => OptionType::Call,
            ColumnSelection::Put => OptionType::Put,
        };

        // Check if we have a position to move
        if let Some(pos_idx) = self.positions.iter().position(|p| p.strike == old_strike && p.kind == kind) {
            let mut pos = self.positions.remove(pos_idx);
            
            // New details
            let new_item = &self.data[new_idx];
            let new_strike = new_item.strike_price;
            let new_ltp = match kind {
                 OptionType::Call => new_item.call_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0),
                 OptionType::Put => new_item.put_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0),
            };

            // Update Position
            pos.strike = new_strike;
            pos.entry_price = new_ltp;

            // Check if target exists
            if let Some(target_pos) = self.positions.iter_mut().find(|p| p.strike == new_strike && p.kind == kind) {
                // Merge
                let total_qty = target_pos.qty + pos.qty;
                if total_qty != 0 {
                    // Avg Price calculation
                    // Value = (OldQty * OldPrice) + (MoveQty * MovePrice)
                    let old_val = target_pos.qty as f64 * target_pos.entry_price;
                    let move_val = pos.qty as f64 * pos.entry_price;
                    target_pos.entry_price = (old_val + move_val) / total_qty as f64;
                    target_pos.qty = total_qty;
                } else {
                    // They cancel out (e.g. +1 and -1)
                    // Remove the target position? 
                    // To do that safely while iterating mutably is hard.
                    // Mark quantity as 0, cleanup later.
                    target_pos.qty = 0;
                }
            } else {
                self.positions.push(pos);
            }
            
            // Cleanup 0 qty
            self.positions.retain(|p| p.qty != 0);
        }

        // Always move cursor
        self.selected_row = new_idx;
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
        if let Some(pos_idx) = self.positions.iter().position(|p| p.strike == strike && p.kind == old_kind) {
            let mut pos = self.positions.remove(pos_idx);
            
            // Update to new kind
            pos.kind = new_kind;
            let new_ltp = match new_kind {
                 OptionType::Call => item.call_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0),
                 OptionType::Put => item.put_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0),
            };
            pos.entry_price = new_ltp;

            // Merge check
            if let Some(target_pos) = self.positions.iter_mut().find(|p| p.strike == strike && p.kind == new_kind) {
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
                self.positions.push(pos);
            }

             self.positions.retain(|p| p.qty != 0);
        }

        // Toggle selection
        self.toggle_column();
    }
}

