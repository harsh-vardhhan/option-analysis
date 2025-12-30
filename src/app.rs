use crate::model::OptionData;

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
}

impl App {
    pub fn new() -> App {
        App {
            data: Vec::new(),
            selected_row: 0,
            selected_column: ColumnSelection::Call,
            should_quit: false,
            initial_centering_done: false,
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
}
