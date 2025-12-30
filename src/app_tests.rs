#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::OptionData;
    use crate::strategy::OptionType;

    #[test]
    fn test_handle_trade_action_put() {
        let mut app = App::new();
        // Mock data
        app.data.push(OptionData {
            expiry: "2023-01-01".to_string(),
            strike_price: 100.0,
            underlying_key: "NIFTY".to_string(),
            underlying_spot_price: 100.0,
            call_options: None,
            put_options: None,
        });

        // Select Put Column
        app.selected_column = ColumnSelection::Put;
        
        // Buy
        app.handle_trade_action(true);

        assert_eq!(app.positions.len(), 1);
        let pos = &app.positions[0];
        assert_eq!(pos.strike, 100.0);
        assert_eq!(pos.kind, OptionType::Put); // This should be Put
        assert_eq!(pos.qty, 1);
    }
}
