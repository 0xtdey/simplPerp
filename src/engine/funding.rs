use crate::user::UserAccount;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub struct FundingRate {
    pub rate: Decimal,
}

impl FundingRate {
    pub fn new() -> Self {
        Self { rate: Decimal::ZERO }
    }

    pub fn calculate_funding(&mut self, mark: Decimal, index: Decimal) -> Decimal {
        // Simple funding: 0.01% per period if mark > index, else -0.01%
        let diff = (mark - index) / index;
        self.rate = diff * dec!(0.1);
        self.rate
    }

    pub fn apply_funding(&self, user: &mut UserAccount, funding: Decimal, mark: Decimal) {
        for pos in user.positions.values_mut() {
            let notional = pos.size * mark;
            let payment = notional * funding;
            match pos.side {
                crate::engine::orderbook::OrderSide::Buy => {
                    user.balance -= payment;
                    user.available_balance -= payment;
                    pos.margin -= payment;
                }
                crate::engine::orderbook::OrderSide::Sell => {
                    user.balance += payment;
                    user.available_balance += payment;
                    pos.margin += payment;
                }
            }
        }
    }
}
