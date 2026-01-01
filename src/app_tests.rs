#[cfg(test)]
mod tests {
    use crate::app::{App, ColumnSelection};
    use crate::model::OptionData;
    use crate::strategy::OptionType;


    fn create_mock_data() -> Vec<OptionData> {
        let mut data = Vec::new();
        // Strikes: 100, 105, 110, 115, 120
        for i in 0..5 {
            data.push(OptionData {
                expiry: "2023-01-01".to_string(),
                strike_price: 100.0 + (i as f64 * 5.0),
                underlying_key: "NIFTY".to_string(),
                underlying_spot_price: 100.0,
                call_options: None, // Simplified, we only care about strike for movement logic
                put_options: None,
            });
        }
        data
    }

    #[test]
    fn test_handle_trade_action_put() {
        let mut app = App::new();
        app.data.push(OptionData {
            expiry: "2023-01-01".to_string(),
            strike_price: 100.0,
            underlying_key: "NIFTY".to_string(),
            underlying_spot_price: 100.0,
            call_options: None,
            put_options: None,
        });

        app.selected_column = ColumnSelection::Put;
        app.handle_trade_action(true);

        assert_eq!(app.positions.len(), 1);
        let pos = &app.positions[0];
        assert_eq!(pos.strike, 100.0);
        assert_eq!(pos.kind, OptionType::Put);
        assert_eq!(pos.qty, 1);
    }
    
    #[test]
    fn test_multi_selection_toggle() {
        let mut app = App::new();
        app.data = create_mock_data(); // 100, 105, 110...
        
        // Select Row 0 (Strike 100), Call Column
        app.selected_row = 0;
        app.selected_column = ColumnSelection::Call;
        
        app.toggle_selection();
        assert!(app.selected_positions.contains(&("100.00".to_string(), OptionType::Call)));
        assert_eq!(app.selected_positions.len(), 1);
        
        // Toggle again should remove
        app.toggle_selection();
        assert!(app.selected_positions.is_empty());
    }

    #[test]
    fn test_multi_move() {
        let mut app = App::new();
        app.data = create_mock_data();
        
        // Setup Positions:
        // Position 1: Long Call 100 (Row 0)
        // Position 2: Short Put 110 (Row 2)
        
        // Add Pos 1
        app.selected_row = 0; // 100
        app.selected_column = ColumnSelection::Call;
        app.handle_trade_action(true); // Buy Call
        
        // Add Pos 2
        app.selected_row = 2; // 110
        app.selected_column = ColumnSelection::Put;
        app.handle_trade_action(false); // Sell Put
        
        assert_eq!(app.positions.len(), 2);
        
        // Select both positions
        // Select Call 100
        app.selected_row = 0;
        app.selected_column = ColumnSelection::Call;
        app.toggle_selection();
        
        // Select Put 110
        app.selected_row = 2;
        app.selected_column = ColumnSelection::Put;
        app.toggle_selection();
        
        assert_eq!(app.selected_positions.len(), 2);
        
        // Move DOWN (+1 index)
        // 100 -> 105
        // 110 -> 115
        app.move_position_row(1);
        
        // Verify Positions
        let call_pos = app.positions.iter().find(|p| p.kind == OptionType::Call).unwrap();
        assert_eq!(call_pos.strike, 105.0);
        
        let put_pos = app.positions.iter().find(|p| p.kind == OptionType::Put).unwrap();
        assert_eq!(put_pos.strike, 115.0);
        
        // Verify Selection Keys Updated
        assert!(app.selected_positions.contains(&("105.00".to_string(), OptionType::Call)));
        assert!(app.selected_positions.contains(&("115.00".to_string(), OptionType::Put)));
        assert!(!app.selected_positions.contains(&("100.00".to_string(), OptionType::Call)));
    }
    
    #[test]
    fn test_multi_move_bounds_check() {
        let mut app = App::new();
        app.data = create_mock_data();
        // Strikes: 100 (idx0), 105 (idx1), 110 (idx2), 115 (idx3), 120 (idx4)
        
        // Position at 120 (idx4). Move Down (+1) should fail.
        app.selected_row = 4;
        app.selected_column = ColumnSelection::Call;
        app.handle_trade_action(true);
        app.toggle_selection();
        
        app.move_position_row(1);
        
        // Should NOT have moved
        let pos = &app.positions[0];
        assert_eq!(pos.strike, 120.0);
        
        // Cursor probably shouldn't move either if it was focused there?
        // Logic says `selected_row` updates if cursor is valid.
        // If cursor is at 4, move 1 -> 5 (invalid). Cursor stays at 4.
        assert_eq!(app.selected_row, 4);
    }
}
