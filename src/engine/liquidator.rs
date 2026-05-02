use crate::engine::matcher;
use crate::engine::orderbook::{OrderBook, OrderSide};
use crate::user::UserAccount;
use rust_decimal::Decimal;

pub struct Liquidator {
    pub maintenance_margin: Decimal,
}

impl Liquidator {
    pub fn new() -> Self {
        Self {
            maintenance_margin: Decimal::from_f64_retain(0.05).unwrap(),
        }
    }
}

pub fn check_liquidations(
    maintenance_margin: Decimal,
    user: &mut UserAccount,
    mark_price: Decimal,
    orderbook: &mut OrderBook,
) {
    let mut to_liquidate = Vec::new();

    user.update_unrealized_pnl(mark_price);

    for (market, pos) in &user.positions {
        let notional = pos.size * mark_price;
        let pnl = pos.unrealized_pnl;
        let margin_ratio = (pos.margin + pnl) / notional;

        if margin_ratio < maintenance_margin {
            to_liquidate.push(market.clone());
        }
    }

    for market in to_liquidate {
        if let Some(pos) = user.positions.remove(&market) {
            let close_side = match pos.side {
                OrderSide::Buy => OrderSide::Sell,
                OrderSide::Sell => OrderSide::Buy,
            };

            let order = crate::engine::orderbook::Order::new(
                user.pubkey.clone(),
                close_side,
                crate::engine::orderbook::OrderType::Market,
                mark_price,
                pos.size,
                pos.leverage,
            );

            let fills = matcher::match_order(orderbook, order);
            for fill in fills {
                user.apply_fill(&fill, mark_price);
            }

            let penalty = pos.margin * Decimal::from_f64_retain(0.01).unwrap();
            user.balance -= penalty;
            user.available_balance -= penalty;
        }
    }
}
