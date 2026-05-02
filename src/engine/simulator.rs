use chrono::{DateTime, Local, TimeZone};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulatedCandle {
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub timestamp_secs: i64,
}

impl SimulatedCandle {
    pub fn timestamp(&self) -> DateTime<Local> {
        Local.timestamp_opt(self.timestamp_secs, 0).single().unwrap_or_else(Local::now)
    }
}

pub struct MarketSimulator {
    pub symbol: String,
    pub seed: u64,
    pub initial_price: f64,
    pub annual_volatility: f64,
    pub annual_drift: f64,
}

impl MarketSimulator {
    /// Generate 30 days of 1-minute OHLCV candles (43,200 candles).
    pub fn generate_history(&self) -> Vec<SimulatedCandle> {
        let mut rng = StdRng::seed_from_u64(self.seed);
        let minutes_per_year = 525_600.0f64;
        let vol_per_min = self.annual_volatility / minutes_per_year.sqrt();
        let drift_per_min = self.annual_drift / minutes_per_year;

        let mut price = self.initial_price;
        let mut candles = Vec::with_capacity(43_200);
        let start_time = Local::now().timestamp() - 30 * 24 * 60 * 60;

        for i in 0..43_200 {
            let open = price;

            // Fat-tailed return: normal with occasional large moves
            let normal = box_muller(&mut rng);
            let tail_scale = 1.0 + 0.25 * normal.abs().powf(0.7);
            let ret = drift_per_min + vol_per_min * normal * tail_scale;

            let close = open * (1.0 + ret);

            // Wicks proportional to volatility and randomness
            let wick_pct = vol_per_min * (0.3 + 0.7 * rng.gen::<f64>());
            let high = f64::max(open, close) * (1.0 + wick_pct);
            let low = f64::min(open, close) * (1.0 - wick_pct);

            // Volume: base + spike on large moves
            let base_vol = 2.0 + rng.gen::<f64>() * 3.0;
            let move_size = (close - open).abs() / open;
            let vol_spike = 50.0 * move_size / vol_per_min;
            let volume = base_vol + vol_spike;

            candles.push(SimulatedCandle {
                open: Decimal::from_f64(open).unwrap_or_default(),
                high: Decimal::from_f64(high).unwrap_or_default(),
                low: Decimal::from_f64(low).unwrap_or_default(),
                close: Decimal::from_f64(close).unwrap_or_default(),
                volume: Decimal::from_f64(volume).unwrap_or_default(),
                timestamp_secs: start_time + i * 60,
            });

            price = close;
        }

        candles
    }

    pub fn save_history(candles: &[SimulatedCandle], path: &Path) -> anyhow::Result<()> {
        let data = serde_json::to_string_pretty(candles)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    pub fn load_history(path: &Path) -> anyhow::Result<Vec<SimulatedCandle>> {
        let data = std::fs::read_to_string(path)?;
        let candles: Vec<SimulatedCandle> = serde_json::from_str(&data)?;
        Ok(candles)
    }

    /// Continue live simulation from the last known price.
    pub fn tick_live(&self, last_price: Decimal, rng: &mut impl Rng) -> (Decimal, Decimal) {
        let price_f = last_price.to_f64().unwrap_or(self.initial_price);
        let minutes_per_year = 525_600.0f64;
        let vol_per_min = self.annual_volatility / minutes_per_year.sqrt();
        let drift_per_min = self.annual_drift / minutes_per_year;

        let normal = box_muller(rng);
        let tail_scale = 1.0 + 0.25 * normal.abs().powf(0.7);
        let ret = drift_per_min + vol_per_min * normal * tail_scale;
        let new_price = price_f * (1.0 + ret);

        let base_vol = 0.5 + rng.gen::<f64>() * 1.5;
        let move_size = (new_price - price_f).abs() / price_f;
        let vol_spike = 50.0 * move_size / vol_per_min;
        let volume = base_vol + vol_spike;

        (
            Decimal::from_f64(new_price).unwrap_or(last_price),
            Decimal::from_f64(volume).unwrap_or(Decimal::ONE),
        )
    }
}

fn box_muller(rng: &mut impl Rng) -> f64 {
    let u1: f64 = rng.gen();
    let u2: f64 = rng.gen();
    // Avoid log(0)
    let u1 = u1.max(1e-10);
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}
