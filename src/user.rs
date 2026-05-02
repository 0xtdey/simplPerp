use crate::engine::matcher::FillEvent;
use crate::engine::orderbook::OrderSide;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserAccount {
    pub pubkey: String,
    pub balance: Decimal,
    pub available_balance: Decimal,
    pub positions: HashMap<String, Position>,
    pub order_history: Vec<OrderHistoryItem>,
    pub fill_history: Vec<FillHistoryItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    pub market: String,
    pub side: OrderSide,
    pub size: Decimal,
    pub entry_price: Decimal,
    pub margin: Decimal,
    pub leverage: u32,
    pub unrealized_pnl: Decimal,
    pub liquidation_price: Decimal,
    pub opened_at: DateTime<Local>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderHistoryItem {
    pub side: OrderSide,
    pub price: Decimal,
    pub size: Decimal,
    pub leverage: u32,
    pub timestamp: DateTime<Local>,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FillHistoryItem {
    pub side: OrderSide,
    pub price: Decimal,
    pub size: Decimal,
    pub pnl: Decimal,
    pub timestamp: DateTime<Local>,
}

impl UserAccount {
    pub fn new() -> Self {
        Self {
            pubkey: generate_pubkey(),
            balance: Decimal::ZERO,
            available_balance: Decimal::ZERO,
            positions: HashMap::new(),
            order_history: Vec::new(),
            fill_history: Vec::new(),
        }
    }

    pub fn deposit(&mut self, amount: Decimal) {
        self.balance += amount;
        self.available_balance += amount;
    }

    pub fn withdraw(&mut self, amount: Decimal) -> bool {
        if self.available_balance >= amount {
            self.balance -= amount;
            self.available_balance -= amount;
            true
        } else {
            false
        }
    }

    pub fn apply_fill(&mut self, fill: &FillEvent, mark_price: Decimal) {
        let market = "BTC-PERP".to_string();
        let pos_key = market.clone();

        let existing = self.positions.get_mut(&pos_key);
        if let Some(pos) = existing {
            // Same side -> increase position
            if pos.side == fill.side {
                let new_size = pos.size + fill.size;
                let total_cost = pos.entry_price * pos.size + fill.price * fill.size;
                pos.entry_price = total_cost / new_size;
                pos.size = new_size;
                pos.margin += fill.margin_used;
                pos.unrealized_pnl = calculate_pnl(pos.side, pos.size, pos.entry_price, mark_price);
                pos.liquidation_price = calculate_liquidation_price(pos.side, pos.entry_price, pos.leverage);
            } else if pos.size <= fill.size {
                // Close / flip
                let pnl = calculate_pnl(pos.side, pos.size, pos.entry_price, fill.price);
                self.balance += pnl;
                self.available_balance += pos.margin + pnl;

                let remaining = fill.size - pos.size;
                if remaining > Decimal::ZERO {
                    let new_pos = Position {
                        market: market.clone(),
                        side: fill.side,
                        size: remaining,
                        entry_price: fill.price,
                        margin: fill.margin_used * (remaining / fill.size),
                        leverage: fill.leverage,
                        unrealized_pnl: Decimal::ZERO,
                        liquidation_price: calculate_liquidation_price(fill.side, fill.price, fill.leverage),
                        opened_at: Local::now(),
                    };
                    self.positions.insert(pos_key.clone(), new_pos);
                } else {
                    self.positions.remove(&pos_key);
                }
            } else {
                // Reduce position
                let pnl = calculate_pnl(pos.side, fill.size, pos.entry_price, fill.price);
                self.balance += pnl;
                self.available_balance += (fill.margin_used) + pnl;
                pos.size -= fill.size;
                pos.margin -= fill.margin_used;
                pos.unrealized_pnl = calculate_pnl(pos.side, pos.size, pos.entry_price, mark_price);
            }
        } else {
            let pos = Position {
                market: market.clone(),
                side: fill.side,
                size: fill.size,
                entry_price: fill.price,
                margin: fill.margin_used,
                leverage: fill.leverage,
                unrealized_pnl: Decimal::ZERO,
                liquidation_price: calculate_liquidation_price(fill.side, fill.price, fill.leverage),
                opened_at: Local::now(),
            };
            self.positions.insert(pos_key, pos);
        }

        self.fill_history.push(FillHistoryItem {
            side: fill.side,
            price: fill.price,
            size: fill.size,
            pnl: self.balance,
            timestamp: Local::now(),
        });
    }

    pub fn update_unrealized_pnl(&mut self, mark_price: Decimal) {
        for pos in self.positions.values_mut() {
            pos.unrealized_pnl = calculate_pnl(pos.side, pos.size, pos.entry_price, mark_price);
        }
    }
}

fn calculate_pnl(side: OrderSide, size: Decimal, entry: Decimal, mark: Decimal) -> Decimal {
    match side {
        OrderSide::Buy => size * (mark - entry),
        OrderSide::Sell => size * (entry - mark),
    }
}

fn calculate_liquidation_price(side: OrderSide, entry: Decimal, leverage: u32) -> Decimal {
    let maint_margin = Decimal::from_f64_retain(0.05).unwrap(); // 5%
    let leverage_dec = Decimal::from(leverage);
    match side {
        OrderSide::Buy => entry * (Decimal::ONE - (Decimal::ONE / leverage_dec) + maint_margin),
        OrderSide::Sell => entry * (Decimal::ONE + (Decimal::ONE / leverage_dec) - maint_margin),
    }
}

fn generate_pubkey() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    bs58::encode(&bytes).into_string()
}
