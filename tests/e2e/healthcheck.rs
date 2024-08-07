use pretty_assertions::assert_eq;
use rstest::rstest;
use testcontainers::ContainerAsync;

use crate::common::containers::{onchain_db::create_onchain_db, Timescale};

#[rstest]
#[tokio::test]
async fn healthcheck_ok(#[future] create_onchain_db: ContainerAsync<Timescale>) {
    let onchain_db = create_onchain_db.await;
    let host_ip = onchain_db.get_host().await.unwrap();
    assert_eq!(host_ip.to_string(), "localhost");
}
