use std::collections::HashMap;

use bigdecimal::{BigDecimal, ToPrimitive};
use deadpool_diesel::postgres::Pool;
use pragma_common::types::DataType;
use pragma_entities::{Currency, EntryError};

use crate::infra::repositories::entry_repository::{
    get_current_median_entries_with_components, MedianEntryWithComponents,
};

pub trait Pricer {
    fn new(pairs: Vec<String>, pair_type: DataType) -> Self;
    async fn compute(&self, db_pool: &Pool) -> Result<Vec<MedianEntryWithComponents>, EntryError>;
}

// =======================================

pub struct IndexPricer {
    pairs: Vec<String>,
    pair_type: DataType,
}

/// Computes the most recent index price for a list of pairs.
/// The index price is the median of the pairs.
impl Pricer for IndexPricer {
    fn new(pairs: Vec<String>, pair_type: DataType) -> Self {
        Self { pairs, pair_type }
    }

    async fn compute(&self, db_pool: &Pool) -> Result<Vec<MedianEntryWithComponents>, EntryError> {
        if self.pairs.is_empty() {
            return Ok(vec![]);
        }
        get_current_median_entries_with_components(db_pool, &self.pairs, self.pair_type)
            .await
            .map_err(|e| e.to_entry_error(&self.pairs.join(",")))
    }
}

// =======================================

/// Computes the mark price for a list of pairs.
/// The mark price can be computed with two methods:
/// 1. if the quote asset is USD, we just return the median price of the recent
///    perp entries.
/// 2. if the quote asset is a stablecoin, we compute the median price of the
///    spot stablecoin/USD pairs and then we divide the median price of the perp
///    pairs by the median price of the stablecoin.
pub struct MarkPricer {
    pairs: Vec<String>,
    pair_type: DataType,
}

impl MarkPricer {
    /// Builds the stablecoin/USD pairs from the non USD pairs.
    /// Example: ["BTC/USDT", "ETH/USDT"] -> ["USDT/USD"]
    fn build_stable_to_usd_pairs(non_usd_pairs: &[String]) -> Vec<String> {
        non_usd_pairs
            .iter()
            .map(|pair| format!("{}/USD", pair.split('/').last().unwrap()))
            .collect()
    }

    /// Computes the stablecoin/USD pairs median entries.
    async fn get_stablecoins_index_entries(
        db_pool: &Pool,
        stablecoin_pairs: &[String],
    ) -> Result<Vec<MedianEntryWithComponents>, EntryError> {
        let stable_to_usd_pairs = Self::build_stable_to_usd_pairs(stablecoin_pairs);
        let stablecoins_index_pricer = IndexPricer::new(stable_to_usd_pairs, DataType::SpotEntry);
        stablecoins_index_pricer.compute(db_pool).await
    }

    /// Retrieves the number of decimals for quote stablecoins.
    /// Example: ["BTC/USDT", "ETH/USDT"] -> {"USDT": 6}
    async fn get_stablecoins_decimals(
        db_pool: &Pool,
        stablecoin_pairs: Vec<String>,
    ) -> Result<HashMap<String, BigDecimal>, EntryError> {
        let conn = db_pool
            .get()
            .await
            .map_err(|_| EntryError::InternalServerError)?;
        let stablecoins_names: Vec<String> = stablecoin_pairs
            .iter()
            // safe unwrap since we know the pairs are formatted "XXX/YYY"
            .map(|pair| pair.split('/').last().unwrap().to_string())
            .collect();
        let decimals = conn
            .interact(move |conn| Currency::get_decimals_for(conn, stablecoins_names))
            .await
            .expect("Couldn't get the decimals for the stablecoins")
            .expect("Couldn't get table result")
            .into_iter()
            .collect();
        Ok(decimals)
    }

    /// Computes the non USD quoted pairs median entries.
    async fn get_pairs_entries(
        db_pool: &Pool,
        pairs: &[String],
        pair_type: DataType,
    ) -> Result<Vec<MedianEntryWithComponents>, EntryError> {
        let pairs_entries = IndexPricer::new(pairs.to_vec(), pair_type);
        pairs_entries.compute(db_pool).await
    }

    /// Given the median price of a perp pair, the median price of the spot
    /// stablecoin/USD pair and the number of decimals of the stablecoin, computes
    /// the mark price.
    fn compute_mark_price(
        perp_pair_price: &BigDecimal,
        spot_usd_price: &BigDecimal,
        decimals: &BigDecimal,
    ) -> BigDecimal {
        let decimals_as_u32 = decimals
            .to_u32()
            .ok_or(EntryError::InternalServerError)
            .unwrap();
        let scaler = BigDecimal::from(10_u32.pow(decimals_as_u32));
        let spot_usd_price = spot_usd_price / scaler;
        perp_pair_price / spot_usd_price
    }

    /// Builds the complete list of entries from the median price of the spot
    /// stablecoin/USD pairs and the median price of the perp pairs.
    pub fn merge_entries_from(
        stablecoins_spot_entries: Vec<MedianEntryWithComponents>,
        stablecoins_decimals: HashMap<String, BigDecimal>,
        pairs_perp_entries: Vec<MedianEntryWithComponents>,
    ) -> Result<Vec<MedianEntryWithComponents>, EntryError> {
        let mut merged_entries = vec![];

        for perp_median_entry in pairs_perp_entries {
            // safe unwrap since we know the pairs are formatted "XXX/YYY"
            let stable_coin_name = perp_median_entry.pair_id.split('/').last().unwrap();
            let related_usd_spot = format!("{}/USD", stable_coin_name);

            let spot_usd_median_entry = stablecoins_spot_entries
                .iter()
                .find(|spot_median_entry| spot_median_entry.pair_id == related_usd_spot)
                .ok_or(EntryError::InternalServerError)?;

            let mark_price = Self::compute_mark_price(
                &perp_median_entry.median_price,
                &spot_usd_median_entry.median_price,
                stablecoins_decimals
                    .get(stable_coin_name)
                    .ok_or(EntryError::InternalServerError)?,
            );

            let mut components = perp_median_entry.components;
            components.extend(spot_usd_median_entry.components.clone());

            let mark_median_entry = MedianEntryWithComponents {
                pair_id: perp_median_entry.pair_id.clone(),
                median_price: mark_price,
                components,
            };
            merged_entries.push(mark_median_entry);
        }

        Ok(merged_entries)
    }
}

impl Pricer for MarkPricer {
    fn new(pairs: Vec<String>, pair_type: DataType) -> Self {
        Self { pairs, pair_type }
    }

    async fn compute(&self, db_pool: &Pool) -> Result<Vec<MedianEntryWithComponents>, EntryError> {
        if self.pairs.is_empty() {
            return Ok(vec![]);
        }
        let (stablecoins_spot_entries, stablecoins_decimals, pairs_perp_entries) = tokio::join!(
            Self::get_stablecoins_index_entries(db_pool, &self.pairs),
            // TODO: currently, we retrieve the decimals everytime for every loop
            // but we should not: they won't change.
            Self::get_stablecoins_decimals(db_pool, self.pairs.clone()),
            Self::get_pairs_entries(db_pool, &self.pairs, self.pair_type)
        );
        Self::merge_entries_from(
            stablecoins_spot_entries?,
            stablecoins_decimals?,
            pairs_perp_entries?,
        )
    }
}
