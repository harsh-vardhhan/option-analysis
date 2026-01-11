use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Span, Line},
    widgets::{Block, Borders, BarChart, Paragraph, Table, Cell, Row},
    Frame,
};
use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Bid/Ask Depth")
        .style(Style::default().fg(Color::Yellow));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    if let Some(depth_data) = &app.market_depth {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Header
                Constraint::Min(0),    // Charts
            ])
            .split(inner_area);

         // 1. Header Info
        let header_text = vec![
            Line::from(vec![
                Span::styled(format!("{} ", depth_data.symbol), Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("LTP: "),
                Span::styled(format!("{:.2} ", depth_data.last_price), Style::default().fg(Color::Cyan)),
                Span::raw("Vol: "),
                Span::styled(format!("{} ", depth_data.volume), Style::default().fg(Color::Gray)),
            ])
        ];
        f.render_widget(Paragraph::new(header_text).alignment(Alignment::Center), chunks[0]);

        // 2. Depth Table
        // We will show Buy (Bid) on Left, Sell (Ask) on Right
        let header_style = Style::default().add_modifier(Modifier::BOLD).fg(Color::DarkGray);
        let header_cells = ["Qty", "Bid", "Ask", "Qty"]
            .iter()
            .map(|h| Cell::from(*h).style(header_style));
        
        // Find max depth layer
        let max_rows = usize::max(depth_data.depth.buy.len(), depth_data.depth.sell.len());
        
        let mut rows = Vec::new();
        for i in 0..max_rows {
            let buy = depth_data.depth.buy.get(i);
            let sell = depth_data.depth.sell.get(i);
            
            // Buy Side
            let (buy_qty, buy_price) = if let Some(b) = buy {
                (format!("{}", b.quantity), format!("{:.2}", b.price))
            } else {
                ("-".to_string(), "-".to_string())
            };
            
            // Sell Side
            let (sell_price, sell_qty) = if let Some(s) = sell {
                (format!("{:.2}", s.price), format!("{}", s.quantity))
            } else {
                ("-".to_string(), "-".to_string())
            };

            let row_cells = vec![
                Cell::from(buy_qty).style(Style::default().fg(Color::Green)),
                Cell::from(buy_price).style(Style::default().fg(Color::Green)),
                Cell::from(sell_price).style(Style::default().fg(Color::Red)),
                Cell::from(sell_qty).style(Style::default().fg(Color::Red)),
            ];
            rows.push(Row::new(row_cells));
        }

        let table = Table::new(rows, [
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .header(Row::new(header_cells).bottom_margin(0))
        .column_spacing(1);

        f.render_widget(table, chunks[1]);

    } else {
        let text = Paragraph::new("Loading Depth...")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(text, inner_area);
    }
}
