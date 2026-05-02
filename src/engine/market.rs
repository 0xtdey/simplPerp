use super::{
    candles::{Candle, CandleChart, Timeframe},
    matcher::FillEvent,
    oracle::Oracle,
    orderbook::{Order, OrderId, OrderBook},
    simulator::{MarketSimulator, SimulatedCandle},
};
use rust_decimal::Decimal;
use std::path::Path;

pub struct Market {
    pub symbol: String,
    pub orderbook: OrderBook,
    pub oracle: Oracle,
    pub chart: CandleChart,
    pub simulator: MarketSimulator,
    pub recent_trades: Vec<super::RecentTrade>,
}

impl Market {
    pub fn new(symbol: &str, simulator: MarketSimulator) -> Self {
        Self {
            symbol: symbol.to_string(),
            orderbook: OrderBook::new(),
            oracle: Oracle::new(),
            chart: CandleChart::new(200),
            simulator,
            recent_trades: Vec::with_capacity(100),
        }
    }

    pub fn seed_orderbook(&mut self, base_price: Decimal) {
        for i in 1..=10 {
            let offset = Decimal::from(i * 10);
            let bid = Order::new(
                "seed".to_string(),
                super::orderbook::OrderSide::Buy,
                super::orderbook::OrderType::Limit,
                base_price - offset,
                Decimal::from(1),
                1,
            );
            let ask = Order::new(
                "seed".to_string(),
                super::orderbook::OrderSide::Sell,
                super::orderbook::OrderType::Limit,
                base_price + offset,
                Decimal::from(1),
                1,
            );
            self.orderbook.add_order(bid);
            self.orderbook.add_order(ask);
        }
    }

    pub fn load_history(&mut self, candles: &[SimulatedCandle]) {
        for sc in candles {
            let candle = Candle {
                open: sc.open,
                high: sc.high,
                low: sc.low,
                close: sc.close,
                volume: sc.volume,
                timestamp: sc.timestamp(),
            };
            self.chart.push_historical(candle);
        }
        // Set oracle to last close price
        if let Some(last) = candles.last() {
            self.oracle.set_price(last.close);
        }
    }

    pub fn submit_order(&mut self, order: Order) -> Vec<FillEvent> {
        super::matcher::match_order(&mut self.orderbook, order)
    }

    pub fn cancel_order(&mut self, order_id: OrderId) -> bool {
        self.orderbook.cancel_order(order_id)
    }

    pub fn add_recent_trade(&mut self, price: Decimal, size: Decimal, side: super::orderbook::OrderSide) {
        self.recent_trades.push(super::RecentTrade {
            price,
            size,
            side,
            timestamp: chrono::Local::now(),
        });
        if self.recent_trades.len() > 100 {
            self.recent_trades.remove(0);
        }
    }
}
