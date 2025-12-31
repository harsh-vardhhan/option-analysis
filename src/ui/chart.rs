use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Chart, Axis, Dataset, GraphType},
    symbols,
    Frame,
};
use crate::app::App;
use crate::strategy::StrategyStats;

pub fn draw(f: &mut Frame, app: &App, stats: &StrategyStats, area: Rect) {
    let spot_price = app.data.first().map(|d| d.underlying_spot_price).unwrap_or(0.0);
    
    // Default bounds if empty: Spot +/- 5%
    let x_min = stats.points.first().map(|p| p.0).unwrap_or(spot_price * 0.95);
    let x_max = stats.points.last().map(|p| p.0).unwrap_or(spot_price * 1.05);

    let x_labels = vec![
        Span::styled(format!("{:.0}", x_min), Style::default().fg(Color::Gray)),
        Span::styled(format!("{:.0}", x_max), Style::default().fg(Color::Gray)),
    ];
    
    let y_min = stats.max_loss.min(0.0) * 1.1; // margin
    let y_max = stats.max_profit.max(0.0) * 1.1;
    
    // If empty, y_min/max are 0.0. Give small range [-1000, 1000] for grid
    let (y_min, y_max) = if app.positions.is_empty() { (-1000.0, 1000.0) } else { (y_min, y_max) };

    let zero_line_data = vec![(x_min, 0.0), (x_max, 0.0)];
    let spot_line_data = vec![(spot_price, y_min), (spot_price, y_max)];

    let datasets = vec![
        Dataset::default()
            .name("Zero")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Gray))
            .data(&zero_line_data),
        Dataset::default()
            .name("Spot")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Blue))
            .data(&spot_line_data),
        Dataset::default()
            .name("P&L")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Yellow))
            .data(&stats.points),
    ];
    
    // Helper for Indian Number System Formatting
    let format_indian = |val: f64| -> String {
        let abs_val = val.abs();
        let int_part = abs_val as u64;
        let s = int_part.to_string();
        let mut bytes = s.into_bytes();
        let len = bytes.len();
        
        let result = if len > 3 {
             let last_three = String::from_utf8(bytes.split_off(len - 3)).unwrap();
             let remaining = String::from_utf8(bytes).unwrap();
             
             let mut groups = Vec::new();
             let r_chars: Vec<char> = remaining.chars().rev().collect();
             for chunk in r_chars.chunks(2) {
                 let g: String = chunk.iter().rev().collect();
                 groups.push(g);
             }
             groups.reverse();
             
             groups.join(",") + "," + &last_three
        } else {
             String::from_utf8(bytes).unwrap()
        };
        
        let sign = if val < 0.0 { "-" } else { "" };
        format!("{}₹{}", sign, result)
    };

    let chart = Chart::new(datasets)
        .block(Block::default().title(" Payoff Graph ").borders(Borders::ALL))
        .x_axis(Axis::default()
            .title("Spot Price")
            .style(Style::default().fg(Color::Gray))
            .bounds([x_min, x_max])
            .labels(x_labels))
        .y_axis(Axis::default()
            .title("P&L")
            .style(Style::default().fg(Color::Gray))
            .bounds([y_min, y_max])
            .labels(vec![
                Span::styled(format_indian(y_min), Style::default().fg(Color::Gray)),
                Span::styled("0", Style::default().fg(Color::Gray)),
                Span::styled(format_indian(y_max), Style::default().fg(Color::Gray)),
            ]));
    
    f.render_widget(chart, area);
}
