use pretty_assertions::assert_eq;
use rstest::rstest;

use crate::common::setup::{TestHelper, setup_containers};

#[rstest]
#[serial_test::serial]
#[tokio::test]
async fn healthcheck_ok(#[future] setup_containers: TestHelper) {
    let mut hlpr = setup_containers.await;

    let body = reqwest::get(hlpr.endpoint("node"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    hlpr.shutdown_local_pragma_node().await;
    assert_eq!(body.trim(), "Server is running!");
}
