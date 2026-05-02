use super::{
    candles::{Candle, CandleChart},
    matcher::FillEvent,
    oracle::Oracle,
    orderbook::{Order, OrderId, OrderBook, OrderSide},
    simulator::{MarketSimulator, SimulatedCandle},
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;

#[derive(Clone, Debug)]
pub struct Market24hStats {
    pub high: Decimal,
    pub low: Decimal,
    pub volume: Decimal,
    pub open_price: Decimal,
    pub prev_close: Decimal,
    pub start_time: chrono::DateTime<chrono::Local>,
}

impl Market24hStats {
    pub fn new(price: Decimal) -> Self {
        Self {
            high: price,
            low: price,
            volume: Decimal::ZERO,
            open_price: price,
            prev_close: price,
            start_time: chrono::Local::now(),
        }
    }

    pub fn update(&mut self, price: Decimal, volume: Decimal) {
        if price > self.high {
            self.high = price;
        }
        if price < self.low {
            self.low = price;
        }
        self.volume += volume;
        self.prev_close = price;
    }

    pub fn change_pct(&self) -> Decimal {
        if self.open_price == Decimal::ZERO {
            return Decimal::ZERO;
        }
        (self.prev_close - self.open_price) / self.open_price * Decimal::from(100)
    }

    pub fn maybe_reset(&mut self) {
        let now = chrono::Local::now();
        if (now - self.start_time).num_hours() >= 24 {
            let p = self.prev_close;
            *self = Self::new(p);
        }
    }
}

pub struct Market {
    pub symbol: String,
    pub orderbook: OrderBook,
    pub oracle: Oracle,
    pub chart: CandleChart,
    pub simulator: MarketSimulator,
    pub recent_trades: Vec<super::RecentTrade>,
    pub stats_24h: Market24hStats,
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
            stats_24h: Market24hStats::new(Decimal::ZERO),
        }
    }

    pub fn seed_orderbook(&mut self, base_price: Decimal) {
        for i in 1..=10 {
            let bid_price = base_price - Decimal::from(i * 10);
            let ask_price = base_price + Decimal::from(i * 10);

            // Multiple orders at same price level for richer depth
            let bid1 = Order::new(
                "seed".to_string(),
                OrderSide::Buy,
                super::orderbook::OrderType::Limit,
                bid_price,
                Decimal::from_f64(0.5 + i as f64 * 0.3).unwrap(),
                1,
            );
            let bid2 = Order::new(
                "seed".to_string(),
                OrderSide::Buy,
                super::orderbook::OrderType::Limit,
                bid_price,
                Decimal::from_f64(1.0 + i as f64 * 0.2).unwrap(),
                1,
            );
            let ask1 = Order::new(
                "seed".to_string(),
                OrderSide::Sell,
                super::orderbook::OrderType::Limit,
                ask_price,
                Decimal::from_f64(0.5 + i as f64 * 0.3).unwrap(),
                1,
            );
            let ask2 = Order::new(
                "seed".to_string(),
                OrderSide::Sell,
                super::orderbook::OrderType::Limit,
                ask_price,
                Decimal::from_f64(1.0 + i as f64 * 0.2).unwrap(),
                1,
            );
            self.orderbook.add_order(bid1);
            self.orderbook.add_order(bid2);
            self.orderbook.add_order(ask1);
            self.orderbook.add_order(ask2);
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
        if let Some(last) = candles.last() {
            self.oracle.set_price(last.close);
            // Initialize 24h stats from the last candle
            self.stats_24h = Market24hStats::new(last.close);
            // Scan back ~24h worth of candles for initial stats
            let cutoff = chrono::Local::now().timestamp() - 24 * 3600;
            for sc in candles.iter().rev() {
                if sc.timestamp_secs < cutoff {
                    break;
                }
                self.stats_24h.high = self.stats_24h.high.max(sc.high);
                self.stats_24h.low = self.stats_24h.low.min(sc.low);
                self.stats_24h.volume += sc.volume;
            }
            if let Some(first_24h) = candles.iter().rev().find(|sc| sc.timestamp_secs >= cutoff) {
                self.stats_24h.open_price = first_24h.open;
            } else if let Some(first) = candles.first() {
                self.stats_24h.open_price = first.open;
            }
        }
    }

    pub fn submit_order(&mut self, order: Order) -> Vec<FillEvent> {
        super::matcher::match_order(&mut self.orderbook, order)
    }

    pub fn cancel_order(&mut self, order_id: OrderId) -> bool {
        self.orderbook.cancel_order(order_id)
    }

    pub fn add_recent_trade(&mut self, price: Decimal, size: Decimal, side: OrderSide) {
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

    pub fn spread(&self) -> Option<(Decimal, Decimal, Decimal)> {
        let best_ask = self.orderbook.best_ask()?;
        let best_bid = self.orderbook.best_bid()?;
        Some((best_bid, best_ask, best_ask - best_bid))
    }
}
