use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Timeframe {
    M1,
    M5,
    M15,
    M60,
    H4,
    D1,
}

impl Timeframe {
    pub fn seconds(&self) -> i64 {
        match self {
            Timeframe::M1 => 60,
            Timeframe::M5 => 300,
            Timeframe::M15 => 900,
            Timeframe::M60 => 3600,
            Timeframe::H4 => 14400,
            Timeframe::D1 => 86400,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Timeframe::M1 => "1m",
            Timeframe::M5 => "5m",
            Timeframe::M15 => "15m",
            Timeframe::M60 => "1h",
            Timeframe::H4 => "4h",
            Timeframe::D1 => "1D",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Timeframe::M1 => Timeframe::M5,
            Timeframe::M5 => Timeframe::M15,
            Timeframe::M15 => Timeframe::M60,
            Timeframe::M60 => Timeframe::H4,
            Timeframe::H4 => Timeframe::D1,
            Timeframe::D1 => Timeframe::M1,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candle {
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub timestamp: DateTime<Local>,
}

impl Candle {
    pub fn new(price: Decimal, timestamp: DateTime<Local>) -> Self {
        Self {
            open: price,
            high: price,
            low: price,
            close: price,
            volume: Decimal::ZERO,
            timestamp,
        }
    }

    pub fn update(&mut self, price: Decimal, volume: Decimal) {
        if price > self.high {
            self.high = price;
        }
        if price < self.low {
            self.low = price;
        }
        self.close = price;
        self.volume += volume;
    }

    #[allow(dead_code)]
    pub fn is_up(&self) -> bool {
        self.close >= self.open
    }

    #[allow(dead_code)]
    pub fn body_size(&self) -> Decimal {
        if self.close >= self.open {
            self.close - self.open
        } else {
            self.open - self.close
        }
    }

    #[allow(dead_code)]
    pub fn wick_high(&self) -> Decimal {
        self.high - if self.close >= self.open { self.close } else { self.open }
    }

    #[allow(dead_code)]
    pub fn wick_low(&self) -> Decimal {
        (if self.close >= self.open { self.open } else { self.close }) - self.low
    }
}

pub struct CandleChart {
    pub timeframe: Timeframe,
    candles: VecDeque<Candle>,
    current: Option<Candle>,
    max_candles: usize,
}

impl CandleChart {
    pub fn new(max_candles: usize) -> Self {
        Self {
            timeframe: Timeframe::M15,
            candles: VecDeque::with_capacity(max_candles),
            current: None,
            max_candles,
        }
    }

    pub fn push_historical(&mut self, candle: Candle) {
        self.candles.push_back(candle);
        if self.candles.len() > self.max_candles {
            self.candles.pop_front();
        }
    }

    pub fn tick(&mut self, price: Decimal, volume: Decimal) {
        let now = Local::now();
        let bucket = Self::floor_timestamp(now, self.timeframe);

        if let Some(ref mut current) = self.current {
            let current_bucket = Self::floor_timestamp(current.timestamp, self.timeframe);
            if bucket != current_bucket {
                let finished = std::mem::replace(current, Candle::new(price, now));
                self.candles.push_back(finished);
                if self.candles.len() > self.max_candles {
                    self.candles.pop_front();
                }
            } else {
                current.update(price, volume);
            }
        } else {
            self.current = Some(Candle::new(price, now));
        }
    }

    pub fn candles(&self) -> Vec<&Candle> {
        let mut result: Vec<_> = self.candles.iter().collect();
        if let Some(ref current) = self.current {
            result.push(current);
        }
        result
    }

    pub fn set_timeframe(&mut self, tf: Timeframe) {
        if self.timeframe != tf {
            self.timeframe = tf;
            self.candles.clear();
            self.current = None;
        }
    }

    #[allow(dead_code)]
    pub fn compute_ema(&self, period: usize) -> Vec<Decimal> {
        let candles = self.candles();
        if candles.len() < period {
            return Vec::new();
        }
        let closes: Vec<Decimal> = candles.iter().map(|c| c.close).collect();
        compute_ema(&closes, period)
    }

    fn floor_timestamp(dt: DateTime<Local>, tf: Timeframe) -> DateTime<Local> {
        let secs = dt.timestamp();
        let bucket_secs = tf.seconds();
        let floored = secs - (secs % bucket_secs);
        DateTime::from_timestamp(floored, 0)
            .map(|t| t.with_timezone(&Local))
            .unwrap_or(dt)
    }
}

#[allow(dead_code)]
pub fn compute_ema(closes: &[Decimal], period: usize) -> Vec<Decimal> {
    if closes.len() < period {
        return Vec::new();
    }

    let multiplier = Decimal::from_f64(2.0 / (period as f64 + 1.0)).unwrap_or(Decimal::ONE);

    // First EMA value is SMA of first `period` closes
    let sma: Decimal = closes[..period].iter().fold(Decimal::ZERO, |a, b| a + *b)
        / Decimal::from(period);

    let mut ema = Vec::with_capacity(closes.len());
    // Pad first `period - 1` positions with zero (we won't render these)
    for _ in 0..period - 1 {
        ema.push(Decimal::ZERO);
    }
    ema.push(sma);

    let mut prev = sma;
    for close in &closes[period..] {
        let val = (*close - prev) * multiplier + prev;
        ema.push(val);
        prev = val;
    }

    ema
}
