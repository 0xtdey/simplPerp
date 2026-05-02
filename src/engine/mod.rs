pub mod candles;
pub mod funding;
pub mod liquidator;
pub mod market;
pub mod matcher;
pub mod oracle;
pub mod orderbook;
pub mod simulator;

use matcher::FillEvent;
use orderbook::{Order, OrderId};
use rust_decimal::Decimal;
use std::collections::HashMap;

pub struct Engine {
    pub markets: HashMap<String, market::Market>,
    pub current_market: String,
    pub funding: funding::FundingRate,
    pub liquidator: liquidator::Liquidator,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct RecentTrade {
    pub price: Decimal,
    pub size: Decimal,
    pub side: orderbook::OrderSide,
    pub timestamp: chrono::DateTime<chrono::Local>,
}

impl Engine {
    pub fn new() -> Self {
        let mut engine = Self {
            markets: HashMap::new(),
            current_market: "BTC-PERP".to_string(),
            funding: funding::FundingRate::new(),
            liquidator: liquidator::Liquidator::new(),
        };

        // Create markets with their simulators
        let btc_sim = simulator::MarketSimulator {
            symbol: "BTC-PERP".to_string(),
            seed: 42,
            initial_price: 45_000.0,
            annual_volatility: 0.60,
            annual_drift: 0.05,
        };
        let mut btc_market = market::Market::new("BTC-PERP", btc_sim);
        btc_market.seed_orderbook(Decimal::from(45000));
        engine.markets.insert("BTC-PERP".to_string(), btc_market);

        let eth_sim = simulator::MarketSimulator {
            symbol: "ETH-PERP".to_string(),
            seed: 43,
            initial_price: 2_400.0,
            annual_volatility: 0.75,
            annual_drift: 0.05,
        };
        let mut eth_market = market::Market::new("ETH-PERP", eth_sim);
        eth_market.seed_orderbook(Decimal::from(2400));
        engine.markets.insert("ETH-PERP".to_string(), eth_market);

        let sol_sim = simulator::MarketSimulator {
            symbol: "SOL-PERP".to_string(),
            seed: 44,
            initial_price: 95.0,
            annual_volatility: 1.10,
            annual_drift: 0.05,
        };
        let mut sol_market = market::Market::new("SOL-PERP", sol_sim);
        sol_market.seed_orderbook(Decimal::from(95));
        engine.markets.insert("SOL-PERP".to_string(), sol_market);

        engine
    }

    pub fn current_market_mut(&mut self) -> &mut market::Market {
        self.markets.get_mut(&self.current_market).unwrap()
    }

    pub fn current_market(&self) -> &market::Market {
        self.markets.get(&self.current_market).unwrap()
    }

    #[allow(dead_code)]
    pub fn switch_market(&mut self, symbol: &str) {
        if self.markets.contains_key(symbol) {
            self.current_market = symbol.to_string();
        }
    }

    pub fn next_market(&mut self) {
        let keys: Vec<_> = self.markets.keys().cloned().collect();
        if let Some(pos) = keys.iter().position(|k| k == &self.current_market) {
            let next = (pos + 1) % keys.len();
            self.current_market = keys[next].clone();
        }
    }

    pub fn submit_order(&mut self, order: Order) -> Vec<FillEvent> {
        let market = self.current_market_mut();
        market.submit_order(order)
    }

    pub fn cancel_order(&mut self, order_id: OrderId) -> bool {
        let market = self.current_market_mut();
        market.cancel_order(order_id)
    }

    pub fn add_recent_trade(&mut self, price: Decimal, size: Decimal, side: orderbook::OrderSide) {
        let market = self.current_market_mut();
        market.add_recent_trade(price, size, side);
    }

    pub fn tick_chart(&mut self, price: Decimal, volume: Decimal) {
        let market = self.current_market_mut();
        market.chart.tick(price, volume);
    }
}
