use crate::app::App;
use ratatui::{
    layout::Constraint,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    if app.show_help {
        let area = crate::ui::centered_rect(60, 60, f.size());
        f.render_widget(ratatui::widgets::Clear, area);

        let block = Block::default()
            .title(" Keyboard Shortcuts ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        // Define rows
        let rows = vec![
            Row::new(vec![
                Cell::from("GENERAL").style(
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Blue),
                ),
                Cell::from(""),
            ]),
            Row::new(vec![
                Cell::from("Shift + S / ?"),
                Cell::from("Toggle this Help Guide"),
            ]),
            Row::new(vec![Cell::from("q"), Cell::from("Quit Application")]),
            Row::new(vec![Cell::from(""), Cell::from("")]), // Spacer
            Row::new(vec![
                Cell::from("NAVIGATION").style(
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Blue),
                ),
                Cell::from(""),
            ]),
            Row::new(vec![
                Cell::from("Arrow Keys"),
                Cell::from("Navigate Grid Cells"),
            ]),
            Row::new(vec![
                Cell::from("Enter"),
                Cell::from("Select Row (if applicable)"),
            ]),
            Row::new(vec![Cell::from(""), Cell::from("")]), // Spacer
            Row::new(vec![
                Cell::from("TRADING").style(
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Blue),
                ),
                Cell::from(""),
            ]),
            Row::new(vec![
                Cell::from("B"),
                Cell::from("Buy Active Selection (+1)"),
            ]),
            Row::new(vec![
                Cell::from("S"),
                Cell::from("Sell Active Selection (-1)"),
            ]),
            Row::new(vec![
                Cell::from("Delete / Backspace"),
                Cell::from("Remove Position at Selection"),
            ]),
            Row::new(vec![Cell::from(""), Cell::from("")]), // Spacer
            Row::new(vec![
                Cell::from("POSITION MANAGEMENT").style(
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Blue),
                ),
                Cell::from(""),
            ]),
            Row::new(vec![
                Cell::from("Space"),
                Cell::from("Select/Deselect Position for Multi-Move"),
            ]),
            Row::new(vec![
                Cell::from("Shift + Arrow Up/Down"),
                Cell::from("Move Selected Position(s) Strike"),
            ]),
            Row::new(vec![
                Cell::from("Shift + Arrow L/R"),
                Cell::from("Switch Expiry Date"),
            ]),
        ];

        let table = Table::new(
            rows,
            [Constraint::Percentage(40), Constraint::Percentage(60)],
        )
        .block(block)
        .column_spacing(1);

        f.render_widget(table, area);
    }
}
