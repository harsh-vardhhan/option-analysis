pub mod bid_ask;
pub mod chart;
pub mod dashboard;
pub mod help;
pub mod setup;
pub mod stats;
pub mod strategies;
pub mod table;
pub mod utils; // [NEW]

pub use utils::{centered_rect, format_indian_currency};

use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App) {
    let constraints = vec![
        Constraint::Length(4),
        Constraint::Min(0),
        Constraint::Length(15),
    ];

    let chunks = Layout::default().constraints(constraints).split(f.size());

    // --- DASHBOARD ---
    dashboard::draw(f, app, chunks[0]);

    // --- MIDDLE SECTION (Table + Strategies + BidAsk) ---
    // Split chunks[1] into Table (75%) and Right Panel (25%)
    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
        .split(chunks[1]);

    // --- TABLE ---
    table::draw(f, app, middle_chunks[0]);

    // --- RIGHT PANEL (Strategies + BidAsk) ---
    // Split right panel (middle_chunks[1]) vertically: 60% Strategies, 40% BidAsk
    let right_panel_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(middle_chunks[1]);

    // Strategies
    strategies::draw(f, app, right_panel_chunks[0]);
    // Bid/Ask
    bid_ask::draw(f, app, right_panel_chunks[1]);

    // --- STRATEGY PANEL ---
    if chunks.len() > 2 {
        // Split the bottom chunk into Left (Stats) and Right (Graph)
        let strategy_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[2]);

        // Calculate Stats
        let stats = app.calculate_strategy_stats();

        // Render Stats
        stats::draw(f, app, &stats, strategy_chunks[0]);

        // Render Chart
        chart::draw(f, app, &stats, strategy_chunks[1]);
    }

    // --- HELP OVERLAY ---
    help::draw(f, app);
}
