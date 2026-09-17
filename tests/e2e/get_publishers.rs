use std::{collections::HashMap, time::Duration};

use deadpool_diesel::postgres::{Manager, Pool};
use diesel::RunQueryDsl;
use pragma_common::{InstrumentType, starknet::StarknetNetwork};
use pragma_node::{
    caches::CacheRegistry,
    handlers::onchain::get_publishers::GetOnchainPublishersParams,
    infra::repositories::onchain_repository::publisher::{
        RawPublisher, get_publishers_with_components,
    },
};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

#[tokio::test]
async fn publisher_history_reuses_a_single_connection() {
    let container;
    let database_url = if let Ok(url) = std::env::var("PRAGMA_TEST_DATABASE_URL") {
        url
    } else {
        container = Postgres::default().start().await.unwrap();
        format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            container.get_host_port_ipv4(5432).await.unwrap()
        )
    };
    let pool = Pool::builder(Manager::new(database_url, deadpool_diesel::Runtime::Tokio1))
        .max_size(1)
        .runtime(deadpool_diesel::Runtime::Tokio1)
        .wait_timeout(Some(Duration::from_secs(1)))
        .build()
        .unwrap();
    {
        let conn = pool.get().await.unwrap();
        conn.interact(|conn| {
            diesel::sql_query(
                "CREATE TEMP TABLE mainnet_spot_entry (
                    pair_id VARCHAR, publisher VARCHAR, source VARCHAR,
                    price NUMERIC, timestamp TIMESTAMPTZ, transaction_hash VARCHAR
                )",
            )
            .execute(conn)
            .unwrap();
            diesel::sql_query(
                "INSERT INTO mainnet_spot_entry VALUES
                    ('ETH/USD', 'PRAGMA', 'BINANCE', 100, NOW(), '0x1'),
                    ('ETH/USD', 'PRAGMA', 'OLD', 90, NOW() - INTERVAL '2 days', '0x2')",
            )
            .execute(conn)
            .unwrap();
        })
        .await
        .unwrap();
    }
    let caches = CacheRegistry::new();
    caches
        .onchain_decimals()
        .insert(
            StarknetNetwork::Mainnet,
            HashMap::from([("ETH/USD".into(), 8)]),
        )
        .await;
    let params = GetOnchainPublishersParams {
        network: StarknetNetwork::Mainnet,
        data_type: InstrumentType::Spot,
        publisher: Some("PRAGMA".into()),
    };
    let rpc_clients = HashMap::new();

    // The cold load needs a second checkout; the warm load must preserve its result.
    for _ in 0..2 {
        let publishers = get_publishers_with_components(
            &pool,
            &params,
            vec![RawPublisher {
                name: "PRAGMA".into(),
                website_url: String::new(),
                publisher_type: 0,
            }],
            &caches,
            &rpc_clients,
        )
        .await
        .expect("publisher history must not hold a connection while acquiring another");
        assert_eq!(publishers.len(), 1);
        assert_eq!(publishers[0].components.len(), 2);
        assert_eq!(publishers[0].components[0].source, "BINANCE");
        assert_eq!(publishers[0].components[0].daily_updates, 1);
        assert_eq!(publishers[0].components[1].source, "OLD");
        assert_eq!(publishers[0].components[1].daily_updates, 0);
        assert_eq!(pool.status().available, 1);
    }
}
