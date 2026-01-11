use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Span, Line},
    widgets::{Block, Borders, Paragraph, Table, Cell, Row},
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
        
        // Find max depth layer and max quantity for scaling
        let max_rows = usize::max(depth_data.depth.buy.len(), depth_data.depth.sell.len());
        
        let max_qty_buy = depth_data.depth.buy.iter().map(|d| d.quantity).max().unwrap_or(1) as f64;
        let max_qty_sell = depth_data.depth.sell.iter().map(|d| d.quantity).max().unwrap_or(1) as f64;
        let max_qty = f64::max(max_qty_buy, max_qty_sell).max(1.0);

        let cell_width = 8; 
        
        // Overlap Colors
        // Darker backgrounds for the bars so white text pops
        let bid_bg = Color::Rgb(40, 80, 40);   // Dark Green BG
        let ask_bg = Color::Rgb(80, 40, 40);   // Dark Red BG
        let text_fg = Color::White;

        let mut rows = Vec::new();
        for i in 0..max_rows {
            let buy = depth_data.depth.buy.get(i);
            let sell = depth_data.depth.sell.get(i);
            
            // Buy Side
            let (buy_cell, buy_price_cell) = if let Some(b) = buy {
                let ratio = (b.quantity as f64 / max_qty).min(1.0);
                let bar_len = (ratio * cell_width as f64).round() as usize;
                
                // Format: Right aligned number "    1200"
                // Bar grows Left -> Right (Inwards to Price at Col 1)
                let text = format!("{:>width$}", b.quantity, width=cell_width);
                
                // Split text into two spans: [Bar Coverage] + [Rest]
                // Since bar grows L->R: 0..bar_len has BG.
                let (part1, part2) = if bar_len >= cell_width {
                    (text.as_str(), "")
                } else {
                    text.split_at(bar_len)
                };

                let span1 = Span::styled(part1.to_string(), Style::default().bg(bid_bg).fg(text_fg));
                let span2 = Span::styled(part2.to_string(), Style::default().fg(Color::DarkGray)); // Empty part dim
                
                (
                    Cell::from(Line::from(vec![span1, span2])),
                    Cell::from(format!("{:.2}", b.price)).style(Style::default().fg(Color::Green))
                )
            } else {
                (Cell::from("-"), Cell::from("-"))
            };
            
            // Sell Side
            let (sell_price_cell, sell_cell) = if let Some(s) = sell {
                let ratio = (s.quantity as f64 / max_qty).min(1.0);
                let bar_len = (ratio * cell_width as f64).round() as usize;

                // Format: Left aligned number "1200    "
                // Bar grows Right -> Left (Inwards to Price at Col 2)
                let text = format!("{:<width$}", s.quantity, width=cell_width);
                
                // Split text: [Rest] + [Bar Coverage]
                let split_idx = cell_width.saturating_sub(bar_len);
                let (part1, part2) = text.split_at(split_idx);
                
                let span1 = Span::styled(part1.to_string(), Style::default().fg(Color::DarkGray)); // Empty part
                let span2 = Span::styled(part2.to_string(), Style::default().bg(ask_bg).fg(text_fg)); // Bar part
                
                (
                    Cell::from(format!("{:.2}", s.price)).style(Style::default().fg(Color::Red)),
                    Cell::from(Line::from(vec![span1, span2]))
                )
            } else {
                (Cell::from("-"), Cell::from("-"))
            };

            rows.push(Row::new(vec![buy_cell, buy_price_cell, sell_price_cell, sell_cell]));
        }

        let table = Table::new(rows, [
            Constraint::Percentage(25), 
            Constraint::Percentage(25), 
            Constraint::Percentage(25), 
            Constraint::Percentage(25), 
        ])
        .header(Row::new(header_cells).bottom_margin(0))
        .column_spacing(2);

        f.render_widget(table, chunks[1]);

    } else {
        let text = Paragraph::new("Loading Depth...")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(text, inner_area);
    }
}
