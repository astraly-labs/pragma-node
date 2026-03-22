use chrono::{DateTime, Utc};
use faucon_rs::consumer::FauConsumerBuilder;
use faucon_rs::topics::FauconTopic;
use faucon_rs::topics::funding_rates::FundingRateFilter;
use faucon_rs::topics::mark_prices::MarkPriceFilter;
use faucon_rs::topics::open_interest::OpenInterestFilter;
use faucon_rs::topics::oracle_prices::OraclePriceFilter;
use faucon_rs::topics::prices::PriceFilter;
use faucon_rs::topics::trades::TradeFilter;
use faucon_rs::{FauconEntry, FauconFilter as _};
use faucon_rs::{consumer::AutoOffsetReset, environment::FauconEnvironment};
use futures_util::StreamExt;
use pragma_common::Pair;
use tokio::sync::mpsc;
use tracing::{error, info};
use uuid::Uuid;

use crate::config::CONFIG;
use crate::entries::{
    FundingRateEntry, OpenInterestEntry, PriceEntry, TradeEntry, instrument_type_str,
    make_market_id,
};

/// Converts milliseconds timestamp to DateTime<Utc>
fn millis_to_datetime(ms: i64) -> DateTime<Utc> {
    DateTime::from_timestamp_millis(ms).unwrap_or_else(Utc::now)
}

/// Normalizes a price value from 18 decimal precision to human-readable format.
/// Divides by 10^18 and formats with appropriate precision.
fn normalize_price(price: u128) -> String {
    const DECIMALS: u32 = 18;
    const DIVISOR: u128 = 10u128.pow(DECIMALS);

    let integer_part = price / DIVISOR;
    let fractional_part = price % DIVISOR;

    if fractional_part == 0 {
        format!("{}.0", integer_part)
    } else {
        // {:018} preserves leading zeros for 18 decimal places
        let frac_str = format!("{:018}", fractional_part);
        let trimmed = frac_str.trim_end_matches('0');
        format!("{}.{}", integer_part, trimmed)
    }
}

/// Runs the Kafka consumer for price entries
pub(crate) async fn run_price_consumer(tx: mpsc::Sender<PriceEntry>) -> anyhow::Result<()> {
    let kafka_environment = FauconEnvironment::Custom(CONFIG.kafka_broker_id.clone());
    let mut consumer = FauConsumerBuilder::on_environment(kafka_environment)
        .group_id(&CONFIG.kafka_group_id)
        .fetch_min_bytes(100_000)
        .fetch_wait_max_ms(25)
        .session_timeout(6000)
        .max_poll_interval(30000)
        .auto_commit(true)
        .auto_commit_interval(1000)
        .max_partition_fetch_bytes(1_048_576)
        .auto_offset_reset(AutoOffsetReset::Latest)
        .build()?;

    consumer.subscribe(&[FauconTopic::PRICES_V2])?;

    info!(
        "Starting price consumer (V2) with {} pairs and {} sources",
        CONFIG.pairs.len(),
        CONFIG.sources.len()
    );

    // Build filter from configured pairs
    let pair_filters: Vec<PriceFilter> = CONFIG
        .pairs
        .iter()
        .filter_map(|p| p.parse::<Pair>().ok().map(PriceFilter::Pair))
        .collect();

    // Build filter from configured sources
    let source_filters: Vec<PriceFilter> = CONFIG
        .sources
        .iter()
        .map(|s| PriceFilter::Source(s.clone()))
        .collect();

    // Combine pair and source filters
    let mut filters = vec![];
    if !pair_filters.is_empty() {
        filters.push(PriceFilter::Any(pair_filters));
    }
    if !source_filters.is_empty() {
        filters.push(PriceFilter::Any(source_filters));
    }

    let price_filter = if filters.is_empty() {
        PriceFilter::All // No filter means accept all
    } else {
        PriceFilter::And(filters)
    };

    let mut stream = consumer.filtered_stream(vec![price_filter.boxed()]);

    loop {
        if let Some(result) = stream.next().await {
            match result {
                Ok(entry) => {
                    if let FauconEntry::Price(entry) = entry {
                        let price_entry = PriceEntry {
                            id: Uuid::new_v4(),
                            market_id: make_market_id(&entry.pair, entry.instrument_type),
                            instrument_type: instrument_type_str(entry.instrument_type),
                            pair_id: entry.pair.to_string(),
                            price: normalize_price(entry.price),
                            exchange_timestamp: millis_to_datetime(entry.timestamp_ms),
                            received_timestamp: millis_to_datetime(entry.received_timestamp_ms),
                            source: entry.source,
                        };

                        if let Err(e) = tx.send(price_entry).await {
                            error!("Failed to send price entry: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to consume price entry: {}", e);
                }
            }
        }
    }
}

/// Runs the Kafka consumer for funding rate entries
pub(crate) async fn run_funding_rate_consumer(
    tx: mpsc::Sender<FundingRateEntry>,
) -> anyhow::Result<()> {
    let kafka_environment = FauconEnvironment::Custom(CONFIG.kafka_broker_id.clone());
    let mut consumer = FauConsumerBuilder::on_environment(kafka_environment)
        .group_id(&CONFIG.kafka_group_id)
        .fetch_min_bytes(100_000)
        .fetch_wait_max_ms(25)
        .session_timeout(6000)
        .max_poll_interval(30000)
        .auto_commit(true)
        .auto_commit_interval(1000)
        .max_partition_fetch_bytes(1_048_576)
        .auto_offset_reset(AutoOffsetReset::Latest)
        .build()?;

    consumer.subscribe(&[FauconTopic::FUNDING_RATES_V2])?;

    info!(
        "Starting funding rate consumer (V2) with {} pairs and {} sources",
        CONFIG.pairs.len(),
        CONFIG.sources.len()
    );

    // Build filter from configured pairs
    let pair_filters: Vec<FundingRateFilter> = CONFIG
        .pairs
        .iter()
        .filter_map(|p| p.parse::<Pair>().ok().map(FundingRateFilter::Pair))
        .collect();

    // Build filter from configured sources
    let source_filters: Vec<FundingRateFilter> = CONFIG
        .sources
        .iter()
        .map(|s| FundingRateFilter::Source(s.clone()))
        .collect();

    // Combine pair and source filters
    let mut filters = vec![];
    if !pair_filters.is_empty() {
        filters.push(FundingRateFilter::Any(pair_filters));
    }
    if !source_filters.is_empty() {
        filters.push(FundingRateFilter::Any(source_filters));
    }

    let funding_rate_filter = if filters.is_empty() {
        FundingRateFilter::All // No filter means accept all
    } else {
        FundingRateFilter::And(filters)
    };

    let mut stream = consumer.filtered_stream(vec![funding_rate_filter.boxed()]);

    loop {
        if let Some(result) = stream.next().await {
            match result {
                Ok(entry) => {
                    if let FauconEntry::FundingRate(entry) = entry {
                        let funding_rate_entry = FundingRateEntry {
                            id: Uuid::new_v4(),
                            market_id: make_market_id(&entry.pair, entry.instrument_type),
                            instrument_type: instrument_type_str(entry.instrument_type),
                            pair_id: entry.pair.to_string(),
                            annualized_rate: entry.annualized_rate,
                            exchange_timestamp: millis_to_datetime(entry.timestamp_ms),
                            received_timestamp: millis_to_datetime(entry.received_timestamp_ms),
                            source: entry.source,
                        };

                        if let Err(e) = tx.send(funding_rate_entry).await {
                            error!("Failed to send funding rate entry: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to consume funding rate entry: {}", e);
                }
            }
        }
    }
}

/// Runs the Kafka consumer for open interest entries
pub(crate) async fn run_open_interest_consumer(
    tx: mpsc::Sender<OpenInterestEntry>,
) -> anyhow::Result<()> {
    let kafka_environment = FauconEnvironment::Custom(CONFIG.kafka_broker_id.clone());
    let mut consumer = FauConsumerBuilder::on_environment(kafka_environment)
        .group_id(&CONFIG.kafka_group_id)
        .fetch_min_bytes(100_000)
        .fetch_wait_max_ms(25)
        .session_timeout(6000)
        .max_poll_interval(30000)
        .auto_commit(true)
        .auto_commit_interval(1000)
        .max_partition_fetch_bytes(1_048_576)
        .auto_offset_reset(AutoOffsetReset::Latest)
        .build()?;

    consumer.subscribe(&[FauconTopic::OPEN_INTEREST_V2])?;

    info!(
        "Starting open interest consumer (V2) with {} pairs and {} sources",
        CONFIG.pairs.len(),
        CONFIG.sources.len()
    );

    // Build filter from configured pairs
    let pair_filters: Vec<OpenInterestFilter> = CONFIG
        .pairs
        .iter()
        .filter_map(|p| p.parse::<Pair>().ok().map(OpenInterestFilter::Pair))
        .collect();

    // Build filter from configured sources
    let source_filters: Vec<OpenInterestFilter> = CONFIG
        .sources
        .iter()
        .map(|s| OpenInterestFilter::Source(s.clone()))
        .collect();

    // Combine pair and source filters
    let mut filters = vec![];
    if !pair_filters.is_empty() {
        filters.push(OpenInterestFilter::Any(pair_filters));
    }
    if !source_filters.is_empty() {
        filters.push(OpenInterestFilter::Any(source_filters));
    }

    let open_interest_filter = if filters.is_empty() {
        OpenInterestFilter::All // No filter means accept all
    } else {
        OpenInterestFilter::And(filters)
    };

    let mut stream = consumer.filtered_stream(vec![open_interest_filter.boxed()]);

    loop {
        if let Some(result) = stream.next().await {
            match result {
                Ok(entry) => {
                    if let FauconEntry::OpenInterest(entry) = entry {
                        let open_interest_entry = OpenInterestEntry {
                            id: Uuid::new_v4(),
                            market_id: make_market_id(&entry.pair, entry.instrument_type),
                            instrument_type: instrument_type_str(entry.instrument_type),
                            pair_id: entry.pair.to_string(),
                            open_interest_value: entry.open_interest,
                            exchange_timestamp: millis_to_datetime(entry.timestamp_ms),
                            received_timestamp: millis_to_datetime(entry.received_timestamp_ms),
                            source: entry.source,
                        };

                        if let Err(e) = tx.send(open_interest_entry).await {
                            error!("Failed to send open interest entry: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to consume open interest entry: {}", e);
                }
            }
        }
    }
}

/// Runs the Kafka consumer for trade entries
pub(crate) async fn run_trade_consumer(tx: mpsc::Sender<TradeEntry>) -> anyhow::Result<()> {
    let kafka_environment = FauconEnvironment::Custom(CONFIG.kafka_broker_id.clone());
    let mut consumer = FauConsumerBuilder::on_environment(kafka_environment)
        .group_id(&CONFIG.kafka_group_id)
        .fetch_min_bytes(100_000)
        .fetch_wait_max_ms(25)
        .session_timeout(6000)
        .max_poll_interval(30000)
        .auto_commit(true)
        .auto_commit_interval(1000)
        .max_partition_fetch_bytes(1_048_576)
        .auto_offset_reset(AutoOffsetReset::Latest)
        .build()?;

    consumer.subscribe(&[FauconTopic::TRADES_V2])?;

    info!(
        "Starting trade consumer (V2) with {} pairs and {} sources",
        CONFIG.pairs.len(),
        CONFIG.sources.len()
    );

    // Build filter from configured pairs
    let pair_filters: Vec<TradeFilter> = CONFIG
        .pairs
        .iter()
        .filter_map(|p| p.parse::<Pair>().ok().map(TradeFilter::Pair))
        .collect();

    // Build filter from configured sources
    let source_filters: Vec<TradeFilter> = CONFIG
        .sources
        .iter()
        .map(|s| TradeFilter::Source(s.clone()))
        .collect();

    // Combine pair and source filters
    let mut filters = vec![];
    if !pair_filters.is_empty() {
        filters.push(TradeFilter::Any(pair_filters));
    }
    if !source_filters.is_empty() {
        filters.push(TradeFilter::Any(source_filters));
    }

    let trades_filter = if filters.is_empty() {
        TradeFilter::All // No filter means accept all
    } else {
        TradeFilter::And(filters)
    };

    let mut stream = consumer.filtered_stream(vec![trades_filter.boxed()]);

    loop {
        if let Some(result) = stream.next().await {
            match result {
                Ok(entry) => {
                    if let FauconEntry::Trade(entry) = entry {
                        let side_str = format!("{:?}", entry.side);
                        let trade_entry = TradeEntry {
                            id: Uuid::new_v4(),
                            market_id: make_market_id(&entry.pair, entry.instrument_type),
                            instrument_type: instrument_type_str(entry.instrument_type),
                            pair_id: entry.pair.to_string(),
                            price: entry.price.to_string(),
                            size: entry.size.to_string(),
                            side: side_str,
                            exchange_timestamp: millis_to_datetime(entry.timestamp_ms),
                            received_timestamp: millis_to_datetime(entry.received_timestamp_ms),
                            source: entry.source,
                            buyer_address: entry.buyer_address,
                            seller_address: entry.seller_address,
                        };

                        if let Err(e) = tx.send(trade_entry).await {
                            error!("Failed to send trade entry: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to consume trade entry: {}", e);
                }
            }
        }
    }
}

/// Runs the Kafka consumer for oracle price entries
pub(crate) async fn run_oracle_price_consumer(tx: mpsc::Sender<PriceEntry>) -> anyhow::Result<()> {
    let kafka_environment = FauconEnvironment::Custom(CONFIG.kafka_broker_id.clone());
    let mut consumer = FauConsumerBuilder::on_environment(kafka_environment)
        .group_id(&CONFIG.kafka_group_id)
        .fetch_min_bytes(100_000)
        .fetch_wait_max_ms(25)
        .session_timeout(6000)
        .max_poll_interval(30000)
        .auto_commit(true)
        .auto_commit_interval(1000)
        .max_partition_fetch_bytes(1_048_576)
        .auto_offset_reset(AutoOffsetReset::Latest)
        .build()?;

    consumer.subscribe(&[FauconTopic::ORACLE_PRICES_V2])?;

    info!(
        "Starting oracle price consumer (V2) with {} pairs and {} sources",
        CONFIG.pairs.len(),
        CONFIG.sources.len()
    );

    let pair_filters: Vec<OraclePriceFilter> = CONFIG
        .pairs
        .iter()
        .filter_map(|p| p.parse::<Pair>().ok().map(OraclePriceFilter::Pair))
        .collect();

    let source_filters: Vec<OraclePriceFilter> = CONFIG
        .sources
        .iter()
        .map(|s| OraclePriceFilter::Source(s.clone()))
        .collect();

    let mut filters = vec![];
    if !pair_filters.is_empty() {
        filters.push(OraclePriceFilter::Any(pair_filters));
    }
    if !source_filters.is_empty() {
        filters.push(OraclePriceFilter::Any(source_filters));
    }

    let oracle_filter = if filters.is_empty() {
        OraclePriceFilter::All
    } else {
        OraclePriceFilter::And(filters)
    };

    let mut stream = consumer.filtered_stream(vec![oracle_filter.boxed()]);

    loop {
        if let Some(result) = stream.next().await {
            match result {
                Ok(entry) => {
                    if let FauconEntry::OraclePrice(entry) = entry {
                        let price_entry = PriceEntry {
                            id: Uuid::new_v4(),
                            market_id: make_market_id(&entry.pair, entry.instrument_type),
                            instrument_type: instrument_type_str(entry.instrument_type),
                            pair_id: entry.pair.to_string(),
                            price: normalize_price(entry.price),
                            exchange_timestamp: millis_to_datetime(entry.timestamp_ms),
                            received_timestamp: millis_to_datetime(entry.received_timestamp_ms),
                            source: entry.source,
                        };

                        if let Err(e) = tx.send(price_entry).await {
                            error!("Failed to send oracle price entry: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to consume oracle price entry: {}", e);
                }
            }
        }
    }
}

/// Runs the Kafka consumer for mark price entries
pub(crate) async fn run_mark_price_consumer(tx: mpsc::Sender<PriceEntry>) -> anyhow::Result<()> {
    let kafka_environment = FauconEnvironment::Custom(CONFIG.kafka_broker_id.clone());
    let mut consumer = FauConsumerBuilder::on_environment(kafka_environment)
        .group_id(&CONFIG.kafka_group_id)
        .fetch_min_bytes(100_000)
        .fetch_wait_max_ms(25)
        .session_timeout(6000)
        .max_poll_interval(30000)
        .auto_commit(true)
        .auto_commit_interval(1000)
        .max_partition_fetch_bytes(1_048_576)
        .auto_offset_reset(AutoOffsetReset::Latest)
        .build()?;

    consumer.subscribe(&[FauconTopic::MARK_PRICES_V2])?;

    info!(
        "Starting mark price consumer (V2) with {} pairs and {} sources",
        CONFIG.pairs.len(),
        CONFIG.sources.len()
    );

    let pair_filters: Vec<MarkPriceFilter> = CONFIG
        .pairs
        .iter()
        .filter_map(|p| p.parse::<Pair>().ok().map(MarkPriceFilter::Pair))
        .collect();

    let source_filters: Vec<MarkPriceFilter> = CONFIG
        .sources
        .iter()
        .map(|s| MarkPriceFilter::Source(s.clone()))
        .collect();

    let mut filters = vec![];
    if !pair_filters.is_empty() {
        filters.push(MarkPriceFilter::Any(pair_filters));
    }
    if !source_filters.is_empty() {
        filters.push(MarkPriceFilter::Any(source_filters));
    }

    let mark_filter = if filters.is_empty() {
        MarkPriceFilter::All
    } else {
        MarkPriceFilter::And(filters)
    };

    let mut stream = consumer.filtered_stream(vec![mark_filter.boxed()]);

    loop {
        if let Some(result) = stream.next().await {
            match result {
                Ok(entry) => {
                    if let FauconEntry::MarkPrice(entry) = entry {
                        let price_entry = PriceEntry {
                            id: Uuid::new_v4(),
                            market_id: make_market_id(&entry.pair, entry.instrument_type),
                            instrument_type: instrument_type_str(entry.instrument_type),
                            pair_id: entry.pair.to_string(),
                            price: normalize_price(entry.price),
                            exchange_timestamp: millis_to_datetime(entry.timestamp_ms),
                            received_timestamp: millis_to_datetime(entry.received_timestamp_ms),
                            source: entry.source,
                        };

                        if let Err(e) = tx.send(price_entry).await {
                            error!("Failed to send mark price entry: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to consume mark price entry: {}", e);
                }
            }
        }
    }
}
