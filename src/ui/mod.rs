pub mod dashboard;
pub mod table;
pub mod stats;
pub mod chart;
pub mod help;
pub mod strategies;
pub mod setup;
pub mod utils;

pub use utils::{centered_rect, format_indian_currency};

use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};
use crate::app::App;

pub fn draw(f: &mut Frame, app: &mut App) {
    let constraints = vec![
        Constraint::Length(4), 
        Constraint::Min(0), 
        Constraint::Length(15)
    ];

    let chunks = Layout::default()
        .constraints(constraints)
        .split(f.size());

    // --- DASHBOARD ---
    dashboard::draw(f, app, chunks[0]);



    // --- MIDDLE SECTION (Table + Strategies) ---
    // Split chunks[1] into Table (80%) and Strategies (20%)
    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .split(chunks[1]);

    // --- TABLE ---
    table::draw(f, app, middle_chunks[0]);
    
    // --- STRATEGIES ---
    strategies::draw(f, app, middle_chunks[1]);

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
