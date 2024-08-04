use httpmock::prelude::*;
use rstest::*;

#[fixture]
pub fn pragmapi_mock() -> MockServer {
    MockServer::start()
}
