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
    // Assuming underlying key has the name, e.g., "NSE_INDEX|Nifty 50"
    let underlying = app.data.first()
        .map(|d| d.underlying_key.split('|').last().unwrap_or(&d.underlying_key))
        .unwrap_or("NIFTY");
    let expiry = app.data.first().map(|d| d.expiry.as_str()).unwrap_or("-");

    let dashboard_text = vec![
        Line::from(vec![
            Span::styled(format!(" {} ", underlying), Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(format!(" {:.2} ", spot_price), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("  Expiry: "),
            Span::styled(expiry, Style::default().fg(Color::Yellow)),
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
