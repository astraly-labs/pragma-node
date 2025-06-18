use crate::common::setup::{TestHelper, setup_containers};
use bigdecimal::BigDecimal;
use chrono::Utc;
use diesel::Connection;
use pragma_entities::{Entry, NewEntry, db::run_migrations};
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn test_transaction_commit_and_rollback(#[future] setup_containers: TestHelper) {
    // Setup test infra and DB
    let test_helper = setup_containers.await;
    let pool = &test_helper.offchain_pool;
    run_migrations(pool).await;

    let conn = pool.get().await.expect("get conn");
    let now = Utc::now().naive_utc();

    // Test commit: should insert successfully
    let entry = NewEntry {
        pair_id: "TEST/COMMIT".to_string(),
        publisher: "test".to_string(),
        source: "test".to_string(),
        timestamp: now,
        price: BigDecimal::from(42),
    };
    let inserted = conn
        .interact(move |conn| Entry::create_one_transactional(conn, entry))
        .await
        .expect("interact")
        .expect("insert");
    assert_eq!(inserted.pair_id, "TEST/COMMIT");

    // Test rollback: should not insert if error occurs
    let entry = NewEntry {
        pair_id: "TEST/ROLLBACK".to_string(),
        publisher: "test".to_string(),
        source: "test".to_string(),
        timestamp: now,
        price: BigDecimal::from(42),
    };
    let result: Result<(), diesel::result::Error> = conn
        .interact(move |conn| {
            conn.transaction(|conn| {
                Entry::create_one(conn, entry)?;
                // Simulate error
                Err(diesel::result::Error::RollbackTransaction)
            })
        })
        .await
        .expect("interact");
    assert!(result.is_err());

    // Confirm rollback: should not find the entry by checking if it exists
    let exists = conn
        .interact(move |conn| Entry::exists(conn, "TEST/ROLLBACK".to_string()))
        .await
        .expect("interact")
        .expect("query");
    assert!(!exists);
}

#[rstest]
#[tokio::test]
async fn test_batch_operations_transactional(#[future] setup_containers: TestHelper) {
    let test_helper = setup_containers.await;
    let pool = &test_helper.offchain_pool;
    run_migrations(pool).await;
    let conn = pool.get().await.expect("get conn");
    let now = Utc::now().naive_utc();

    let entries: Vec<NewEntry> = (0..5)
        .map(|i| NewEntry {
            pair_id: format!("BATCH/{}", i),
            publisher: "batch_test".to_string(),
            source: "batch_test".to_string(),
            timestamp: now,
            price: BigDecimal::from(i * 10 + 1),
        })
        .collect();

    let inserted = conn
        .interact(move |conn| Entry::create_many_transactional(conn, entries))
        .await
        .expect("interact")
        .expect("insert");
    assert_eq!(inserted.len(), 5);
}

#[rstest]
#[tokio::test]
async fn test_transaction_performance(#[future] setup_containers: TestHelper) {
    use std::time::Instant;
    let test_helper = setup_containers.await;
    let pool = &test_helper.offchain_pool;
    run_migrations(pool).await;
    let conn = pool.get().await.expect("get conn");
    let now = Utc::now().naive_utc();

    let entries: Vec<NewEntry> = (0..100)
        .map(|i| NewEntry {
            pair_id: format!("PERF/{}", i),
            publisher: "perf_test".to_string(),
            source: "perf_test".to_string(),
            timestamp: now,
            price: BigDecimal::from(i * 100 + 1),
        })
        .collect();

    // Batch insert with transaction
    let start = Instant::now();
    let _ = conn
        .interact(move |conn| Entry::create_many_transactional(conn, entries))
        .await
        .expect("interact")
        .expect("insert");
    let duration = start.elapsed();
    println!("Batch insert with transaction took: {:?}", duration);
    // (Optional) You can add assertions or log/compare with non-transactional if desired
}
