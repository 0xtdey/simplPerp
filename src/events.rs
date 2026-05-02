use crossterm::event;
use rust_decimal::Decimal;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum Event {
    Tick,
    Crossterm(event::Event),
    MarketUpdate(Decimal),
}

pub struct EventHandler {
    tx: mpsc::UnboundedSender<Event>,
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tx: mpsc::UnboundedSender<Event>, tick_rate: Duration) -> Self {
        Self { tx, tick_rate }
    }

    pub fn spawn(self) {
        let tx = self.tx.clone();
        let tick_rate = self.tick_rate;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tick_rate);
            loop {
                interval.tick().await;
                if tx.send(Event::Tick).is_err() {
                    break;
                }
            }
        });

        let tx = self.tx.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(evt) = event::read() {
                    if tx.send(Event::Crossterm(evt)).is_err() {
                        break;
                    }
                }
            }
        });
    }
}
