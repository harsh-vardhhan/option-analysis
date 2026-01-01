use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use crate::app::{App, Focus};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app.strategies
        .iter()
        .map(|s| {
            ListItem::new(*s).style(Style::default().fg(Color::White))
        })
        .collect();

    // Determine Border Color based on Focus
    let border_color = if app.active_focus == Focus::Strategies {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Strategies ").border_style(Style::default().fg(border_color)))
        .highlight_style(
            Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );
        
    // We need a ListState to render selected item
    let mut state = ListState::default();
    state.select(Some(app.selected_strategy));

    f.render_stateful_widget(items, area, &mut state);
}
