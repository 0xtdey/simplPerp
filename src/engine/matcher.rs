use super::orderbook::{Order, OrderBook, OrderSide, OrderType};
use rust_decimal::Decimal;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct FillEvent {
    pub order_id: u64,
    pub user: String,
    pub side: OrderSide,
    pub price: Decimal,
    pub size: Decimal,
    pub margin_used: Decimal,
    pub leverage: u32,
}

pub fn match_order(book: &mut OrderBook, mut order: Order) -> Vec<FillEvent> {
    let mut fills = Vec::new();

    match order.side {
        OrderSide::Buy => {
            while order.remaining > Decimal::ZERO {
                let best_ask = match book.best_ask() {
                    Some(p) => p,
                    None => break,
                };

                if order.order_type == OrderType::Limit && order.price < best_ask {
                    break;
                }

                let price = best_ask;
                let mut consumed = false;

                book.iter_asks(|o| {
                    if o.price != price || order.remaining <= Decimal::ZERO {
                        return false;
                    }
                    let fill_size = o.remaining.min(order.remaining);
                    let notional = fill_size * price;
                    let margin = notional / Decimal::from(order.leverage);

                    fills.push(FillEvent {
                        order_id: order.id,
                        user: order.user.clone(),
                        side: order.side,
                        price,
                        size: fill_size,
                        margin_used: margin,
                        leverage: order.leverage,
                    });

                    o.remaining -= fill_size;
                    order.remaining -= fill_size;
                    consumed = o.remaining <= Decimal::ZERO;
                    consumed
                });
            }
        }
        OrderSide::Sell => {
            while order.remaining > Decimal::ZERO {
                let best_bid = match book.best_bid() {
                    Some(p) => p,
                    None => break,
                };

                if order.order_type == OrderType::Limit && order.price > best_bid {
                    break;
                }

                let price = best_bid;
                let mut consumed = false;

                book.iter_bids(|o| {
                    if o.price != price || order.remaining <= Decimal::ZERO {
                        return false;
                    }
                    let fill_size = o.remaining.min(order.remaining);
                    let notional = fill_size * price;
                    let margin = notional / Decimal::from(order.leverage);

                    fills.push(FillEvent {
                        order_id: order.id,
                        user: order.user.clone(),
                        side: order.side,
                        price,
                        size: fill_size,
                        margin_used: margin,
                        leverage: order.leverage,
                    });

                    o.remaining -= fill_size;
                    order.remaining -= fill_size;
                    consumed = o.remaining <= Decimal::ZERO;
                    consumed
                });
            }
        }
    }

    if order.remaining > Decimal::ZERO && order.order_type == OrderType::Limit {
        book.add_order(order);
    }

    fills
}
