use aws_sdk_secretsmanager::Client;

#[derive(Debug)]
pub enum AwsError {
    NoSecretFound,
}

pub async fn get_aws_client() -> Client {
    let aws_config = aws_config::load_from_env().await;
    aws_sdk_secretsmanager::Client::new(&aws_config)
}

pub async fn get_aws_secret(client: &Client, secret_name: &str) -> Result<String, AwsError> {
    let response = client
        .get_secret_value()
        .secret_id(secret_name)
        .send()
        .await;
    match response {
        Ok(wrapped_secret) => {
            let secret = wrapped_secret
                .secret_string
                .expect("No secret string found");
            Ok(secret)
        }
        Err(_) => Err(AwsError::NoSecretFound),
    }
}
