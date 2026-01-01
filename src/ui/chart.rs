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
    let (y_min, y_max) = if app.portfolio.positions.is_empty() { (-1000.0, 1000.0) } else { (y_min, y_max) };

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
    
    // Helper removed, using crate::ui::format_indian_currency

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
                Span::styled(crate::ui::format_indian_currency(y_min), Style::default().fg(Color::Gray)),
                Span::styled("0", Style::default().fg(Color::Gray)),
                Span::styled(crate::ui::format_indian_currency(y_max), Style::default().fg(Color::Gray)),
            ]));
    
    f.render_widget(chart, area);
}
