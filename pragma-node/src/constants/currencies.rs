/// We used to have a `Currencies` table with abstract currencies.
/// We removed it - for now we just store them in this constant since we don't
/// update this often at all.
pub const ABSTRACT_CURRENCIES: [&str; 4] = ["USD", "EUR", "BTC", "USDPLUS"];
