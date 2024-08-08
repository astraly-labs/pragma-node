use pretty_assertions::assert_eq;
use rstest::rstest;

use crate::common::setup::{setup_containers, TestHelper};

#[rstest]
#[tokio::test]
async fn healthcheck_ok(#[future] setup_containers: TestHelper) {
    let _c = setup_containers.await;

    let body = reqwest::get("http://localhost:3000/node".to_string())
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert_eq!(body.trim(), "Server is running!");
}
