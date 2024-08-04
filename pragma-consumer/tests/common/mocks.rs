use httpmock::prelude::*;
use rstest::*;

#[fixture]
pub fn pragmapi_mock() -> MockServer {
    let pragmapi = MockServer::start();

    // Mock the healthcheck endpoint
    let _ = pragmapi.mock(|when, then| {
        when.method(GET).path("/node");
        then.status(200)
            .header("content-type", "text/html")
            .body("Server is running!");
    });

    pragmapi
}
