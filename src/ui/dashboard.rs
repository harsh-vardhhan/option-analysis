use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},

    Frame,
};
use ratatui::layout::Alignment;
use crate::app::App;

// function removed

// Redefining to match the calling pattern in mod.rs
pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let spot_price = app.data.first().map(|d| d.underlying_spot_price).unwrap_or(0.0);
    let underlying = app.data.first()
        .map(|d| d.underlying_key.split('|').next_back().unwrap_or(&d.underlying_key))
        .unwrap_or("NIFTY");
    
    // Use selected expiry from App state
    let expiry_display = if let Some(exp) = app.available_expiries.get(app.current_expiry_index) {
        // Add arrows to indicate switchability
        let left_arrow = if app.current_expiry_index > 0 { "< " } else { "  " };
        let right_arrow = if app.current_expiry_index < app.available_expiries.len() - 1 { " >" } else { "  " };
        format!("{}{}{}", left_arrow, exp, right_arrow)
    } else {
        String::from("-")
    };

    let dashboard_text = vec![
        Line::from(vec![
            Span::styled(format!(" {} ", underlying), Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(format!(" {:.2} ", spot_price), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("  Expiry: "),
            Span::styled(expiry_display, Style::default().fg(Color::Yellow)),
            Span::raw("  |  Last Action: "),
            Span::styled(format!(" {} ", app.last_message), Style::default().bg(Color::DarkGray).fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("LIVE MARKET DATA", Style::default().fg(Color::Green).add_modifier(Modifier::RAPID_BLINK)), // subtle blinking
            Span::raw(" • Press 'q' to quit • Arrow keys to navigate"),
            Span::raw(" • "),
            Span::styled("Shortcuts: Shift + S", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        ])
    ];

    let dashboard = Paragraph::new(dashboard_text)
        .block(Block::default().borders(Borders::ALL).title(" Dashboard "))
        .alignment(Alignment::Center);
    f.render_widget(dashboard, area);
}
