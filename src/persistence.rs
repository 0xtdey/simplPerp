use crate::app::{App, Screen, TradeField};
use crate::user::UserAccount;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct SavedState {
    current_screen: usize,
    user: UserAccount,
    trade_form: Option<SavedTradeForm>,
}

#[derive(Serialize, Deserialize)]
struct SavedTradeForm {
    side: String,
    order_type: String,
    price: String,
    size: String,
    leverage: String,
    field: usize,
}

pub fn load(path: &Path) -> anyhow::Result<App> {
    let data = std::fs::read_to_string(path)?;
    let saved: SavedState = serde_json::from_str(&data)?;

    let mut app = App::new(path.to_path_buf());
    app.user = saved.user;

    match saved.current_screen {
        0 => app.current_screen = Screen::Market,
        1 => app.current_screen = Screen::Trade,
        2 => app.current_screen = Screen::Positions,
        3 => app.current_screen = Screen::Account,
        4 => app.current_screen = Screen::History,
        5 => app.current_screen = Screen::Help,
        _ => {}
    }

    if let Some(tf) = saved.trade_form {
        app.trade_form.side = if tf.side == "Buy" { crate::engine::orderbook::OrderSide::Buy } else { crate::engine::orderbook::OrderSide::Sell };
        app.trade_form.order_type = if tf.order_type == "Market" { crate::engine::orderbook::OrderType::Market } else { crate::engine::orderbook::OrderType::Limit };
        app.trade_form.price = tf.price;
        app.trade_form.size = tf.size;
        app.trade_form.leverage = tf.leverage;
        app.trade_form.field = match tf.field {
            0 => TradeField::Side,
            1 => TradeField::Type,
            2 => TradeField::Price,
            3 => TradeField::Size,
            _ => TradeField::Leverage,
        };
    }

    Ok(app)
}

pub fn save(app: &App) -> anyhow::Result<()> {
    let screen_idx = match app.current_screen {
        Screen::Market => 0,
        Screen::Trade => 1,
        Screen::Positions => 2,
        Screen::Account => 3,
        Screen::History => 4,
        Screen::Help => 5,
    };

    let saved = SavedState {
        current_screen: screen_idx,
        user: app.user.clone(),
        trade_form: Some(SavedTradeForm {
            side: format!("{:?}", app.trade_form.side),
            order_type: format!("{:?}", app.trade_form.order_type),
            price: app.trade_form.price.clone(),
            size: app.trade_form.size.clone(),
            leverage: app.trade_form.leverage.clone(),
            field: match app.trade_form.field {
                TradeField::Side => 0,
                TradeField::Type => 1,
                TradeField::Price => 2,
                TradeField::Size => 3,
                TradeField::Leverage => 4,
            },
        }),
    };

    let data = serde_json::to_string_pretty(&saved)?;
    std::fs::write(&app.state_path, data)?;
    Ok(())
}
