use pretty_assertions::assert_eq;
use rstest::rstest;

use crate::common::containers::setup_containers;
use crate::common::containers::Containers;

#[rstest]
#[tokio::test]
async fn healthcheck_ok(#[future] setup_containers: Containers) {
    let c = setup_containers.await;

    let body = reqwest::get(format!("{}/node", c.pragma_node.base_url()))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert_eq!(body.trim(), "Server is running!");
}
