use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub struct Oracle {
    price: Decimal,
    index_price: Decimal,
}

impl Oracle {
    pub fn new() -> Self {
        Self {
            price: dec!(50000),
            index_price: dec!(50000),
        }
    }

    pub fn price(&self) -> Decimal {
        self.price
    }

    pub fn index_price(&self) -> Decimal {
        self.index_price
    }

    pub fn set_price(&mut self, price: Decimal) {
        self.price = price;
        // Index tracks mark with slight lag
        self.index_price = self.index_price * dec!(0.999) + price * dec!(0.001);
    }
}
