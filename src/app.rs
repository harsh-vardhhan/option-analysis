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
    // Key: (Strike, OptionType) -> Position
    // Actually we need to support multiple legs, but usually one leg per strike/type is enough for builders.
    // We'll use a Vec for simplicity in rendering linear list, but Map for fast lookup?
    // Let's use a Vec and simple linear scan for small N.
    pub positions: Vec<Position>,
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

    pub fn handle_trade_action(&mut self, is_buy: bool) {
        if self.data.is_empty() { return; }
        
        let item = &self.data[self.selected_row];
        let strike = item.strike_price;
        let (kind, price) = match self.selected_column {
            ColumnSelection::Call => (OptionType::Call, item.call_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0)),
            ColumnSelection::Put => (OptionType::Put, item.put_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0)),
        };

        // Find existing position
        if let Some(pos) = self.positions.iter_mut().find(|p| p.strike == strike && p.kind == kind) {
            // Flip logic: 
            // If Buy requested:
            //    If current Qty < 0 (Short), removing shorts -> Incr Qty
            //    If current Qty > 0 (Long), adding longs -> Incr Qty
            // Wait, usually "Buy" button just adds +1 qty, "Sell" adds -1 qty.
            // User requirement: "if I presse S on a strike which has B's, it should remove all Bs and start calculating for S"
            
            if is_buy {
                if pos.qty < 0 {
                     // Was Short, switch to Long +1 immediately? Or unwind?
                     // "Remove all Bs" implies clearing opposing.
                     // "start calculating for S" implies switching direction.
                     
                     // Strict Interpretation:
                     // If I have Sell (-N), and press Buy:
                     // Reset to Buy (+1).
                     pos.qty = 1;
                     pos.entry_price = price; // Update price to current
                } else {
                    pos.qty += 1;
                    // Avg price logic? Simplified: keep latest or average?
                    // Strategy Builders usually simulate "Current Market Entry". So updating price is fine, or avg.
                    // Let's weighted average for realism if adding to same side.
                    // new_price = (old_total + new_price) / new_qty
                    let old_total = pos.entry_price * (pos.qty - 1) as f64;
                    pos.entry_price = (old_total + price) / pos.qty as f64;
                }
            } else {
                // Sell requested
                if pos.qty > 0 {
                    // Was Long, switch to Short -1
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
    }
}
