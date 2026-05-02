use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type OrderId = u64;

static mut NEXT_ORDER_ID: u64 = 1;

fn next_order_id() -> OrderId {
    unsafe {
        let id = NEXT_ORDER_ID;
        NEXT_ORDER_ID += 1;
        id
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub user: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Decimal,
    pub size: Decimal,
    pub remaining: Decimal,
    pub leverage: u32,
    pub created_at: DateTime<Local>,
}

impl Order {
    pub fn new(
        user: String,
        side: OrderSide,
        order_type: OrderType,
        price: Decimal,
        size: Decimal,
        leverage: u32,
    ) -> Self {
        Self {
            id: next_order_id(),
            user,
            side,
            order_type,
            price,
            size,
            remaining: size,
            leverage,
            created_at: Local::now(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct OrderBook {
    bids: BTreeMap<Decimal, Vec<Order>>,
    asks: BTreeMap<Decimal, Vec<Order>>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    pub fn add_order(&mut self, order: Order) {
        let book = match order.side {
            OrderSide::Buy => &mut self.bids,
            OrderSide::Sell => &mut self.asks,
        };
        book.entry(order.price).or_default().push(order);
    }

    pub fn cancel_order(&mut self, order_id: OrderId) -> bool {
        for orders in self.bids.values_mut() {
            if let Some(pos) = orders.iter().position(|o| o.id == order_id) {
                orders.remove(pos);
                return true;
            }
        }
        for orders in self.asks.values_mut() {
            if let Some(pos) = orders.iter().position(|o| o.id == order_id) {
                orders.remove(pos);
                return true;
            }
        }
        false
    }

    pub fn cancel_random_order(&mut self, side: OrderSide, price: Decimal) -> bool {
        let book = match side {
            OrderSide::Buy => &mut self.bids,
            OrderSide::Sell => &mut self.asks,
        };
        if let Some(orders) = book.get_mut(&price) {
            if !orders.is_empty() {
                orders.remove(0);
                if orders.is_empty() {
                    book.remove(&price);
                }
                return true;
            }
        }
        false
    }

    pub fn best_bid(&self) -> Option<Decimal> {
        self.bids.keys().max().copied()
    }

    pub fn best_ask(&self) -> Option<Decimal> {
        self.asks.keys().min().copied()
    }

    pub fn bid_prices(&self) -> Vec<Decimal> {
        self.bids.keys().copied().collect()
    }

    pub fn ask_prices(&self) -> Vec<Decimal> {
        self.asks.keys().copied().collect()
    }

    pub fn l2_snapshot(&self, depth: usize) -> (Vec<(Decimal, Decimal)>, Vec<(Decimal, Decimal)>) {
        let mut bids: Vec<_> = self
            .bids
            .iter()
            .map(|(price, orders)| {
                let total = orders.iter().map(|o| o.remaining).sum();
                (*price, total)
            })
            .collect();
        bids.sort_by(|a, b| b.0.cmp(&a.0));
        bids.truncate(depth);

        let mut asks: Vec<_> = self
            .asks
            .iter()
            .map(|(price, orders)| {
                let total = orders.iter().map(|o| o.remaining).sum();
                (*price, total)
            })
            .collect();
        asks.sort_by(|a, b| a.0.cmp(&b.0));
        asks.truncate(depth);

        (bids, asks)
    }

    pub fn user_orders(&self, user: &str) -> Vec<Order> {
        let mut result = Vec::new();
        for orders in self.bids.values() {
            for o in orders {
                if o.user == user {
                    result.push(o.clone());
                }
            }
        }
        for orders in self.asks.values() {
            for o in orders {
                if o.user == user {
                    result.push(o.clone());
                }
            }
        }
        result
    }

    #[allow(dead_code)]
    pub fn remove_order(&mut self, side: OrderSide, price: Decimal, order_id: OrderId) -> Option<Order> {
        let book = match side {
            OrderSide::Buy => &mut self.bids,
            OrderSide::Sell => &mut self.asks,
        };
        if let Some(orders) = book.get_mut(&price) {
            if let Some(pos) = orders.iter().position(|o| o.id == order_id) {
                return Some(orders.remove(pos));
            }
        }
        None
    }

    pub fn iter_bids<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Order) -> bool,
    {
        let prices: Vec<_> = self.bids.keys().copied().collect();
        for price in prices {
            if let Some(orders) = self.bids.get_mut(&price) {
                orders.retain_mut(|o| !f(o));
                if orders.is_empty() {
                    self.bids.remove(&price);
                }
            }
        }
    }

    pub fn iter_asks<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Order) -> bool,
    {
        let prices: Vec<_> = self.asks.keys().copied().collect();
        for price in prices {
            if let Some(orders) = self.asks.get_mut(&price) {
                orders.retain_mut(|o| !f(o));
                if orders.is_empty() {
                    self.asks.remove(&price);
                }
            }
        }
    }
}
