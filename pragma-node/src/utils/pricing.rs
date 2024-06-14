use deadpool_diesel::postgres::Pool;
use pragma_common::types::DataType;
use pragma_entities::EntryError;

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

pub struct MarkPricer {
    pairs: Vec<String>,
    pair_type: DataType,
}

impl MarkPricer {
    fn build_stable_to_usd_pairs(non_usd_pairs: Vec<String>) -> Vec<String> {
        non_usd_pairs
            .iter()
            .map(|pair| format!("{}/USD", pair.split('/').last().unwrap()))
            .collect()
    }

    pub async fn get_stablecoins_index_entries(
        db_pool: &Pool,
        stablecoin_pairs: Vec<String>,
    ) -> Result<Vec<MedianEntryWithComponents>, EntryError> {
        let stable_to_usd_pairs = Self::build_stable_to_usd_pairs(stablecoin_pairs);
        let stablecoins_index_pricer = IndexPricer::new(stable_to_usd_pairs, DataType::SpotEntry);
        stablecoins_index_pricer.compute(db_pool).await
    }

    pub async fn get_pairs_entries(
        db_pool: &Pool,
        pairs: Vec<String>,
        pair_type: DataType,
    ) -> Result<Vec<MedianEntryWithComponents>, EntryError> {
        let pairs_entries = IndexPricer::new(pairs, pair_type);
        pairs_entries.compute(db_pool).await
    }

    pub fn merge_entries_from(
        stablecoins_spot_entries: Vec<MedianEntryWithComponents>,
        pairs_perp_entries: Vec<MedianEntryWithComponents>,
    ) -> Result<Vec<MedianEntryWithComponents>, EntryError> {
        let mut merged_entries = vec![];

        for perp_median_entry in pairs_perp_entries {
            let related_usd_spot = format!(
                "{}/USD",
                // TODO: unsafe unwrap
                perp_median_entry.pair_id.split('/').last().unwrap()
            );

            let spot_usd_median_entry = stablecoins_spot_entries
                .iter()
                .find(|spot_median_entry| spot_median_entry.pair_id == related_usd_spot)
                .ok_or(EntryError::InternalServerError)?;

            let perp_pair_price = perp_median_entry.median_price.clone();
            let spot_usd_price = spot_usd_median_entry.median_price.clone();
            let mark_price = perp_pair_price / spot_usd_price;

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
        let (stablecoins_spot_entries, pairs_perp_entries) = tokio::join!(
            Self::get_stablecoins_index_entries(db_pool, self.pairs.clone()),
            Self::get_pairs_entries(db_pool, self.pairs.clone(), self.pair_type)
        );
        Self::merge_entries_from(stablecoins_spot_entries?, pairs_perp_entries?)
    }
}
