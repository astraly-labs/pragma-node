use pretty_assertions::assert_eq;
use rstest::rstest;

use crate::common::setup::{setup_containers, TestHelper};

#[rstest]
#[serial_test::serial]
#[tokio::test]
async fn healthcheck_ok(#[future] setup_containers: TestHelper) {
    let hlpr = setup_containers.await;

    let body = reqwest::get(hlpr.endpoint("node"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert_eq!(body.trim(), "Server is running!");
}
