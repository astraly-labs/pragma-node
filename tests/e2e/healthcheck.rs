use rstest::rstest;
use pretty_assertions::assert_eq;

#[rstest]
fn healthcheck_ok() {
    assert_eq!(1, 0);
}
