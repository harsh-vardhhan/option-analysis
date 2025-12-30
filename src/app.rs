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
}

