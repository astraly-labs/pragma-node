mod aws;
mod gcp;

use starknet::signers::SigningKey;

use crate::config::CloudEnv;

use aws::PragmaSignerBuilder as AwsPragmaSignerBuilder;
use gcp::PragmaSignerBuilder as GcpPragmaSignerBuilder;

pub async fn build_signer(cloud_env: CloudEnv, is_production: bool) -> Option<SigningKey> {
    let cloud_signer = if is_production {
        match cloud_env {
            CloudEnv::Aws => {
                AwsPragmaSignerBuilder::new()
                    .production_mode()
                    .build()
                    .await
            }
            CloudEnv::Gcp => {
                GcpPragmaSignerBuilder::new()
                    .production_mode()
                    .build()
                    .await
            }
        }
    } else {
        match cloud_env {
            CloudEnv::Aws => {
                AwsPragmaSignerBuilder::new()
                    .non_production_mode()
                    .build()
                    .await
            }
            CloudEnv::Gcp => {
                GcpPragmaSignerBuilder::new()
                    .non_production_mode()
                    .build()
                    .await
            }
        }
    };
    println!("do we have prod: {}", is_production);
    if !is_production && cloud_signer.is_none() {
        tracing::info!(
            "Not in production mode and no cloud signer found. Generating random signing key for development."
        );
        let random_key = SigningKey::from_random();
        Some(random_key)
    } else {
        cloud_signer
    }
}
