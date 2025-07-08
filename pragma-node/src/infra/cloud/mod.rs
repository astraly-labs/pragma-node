mod aws;
mod gcp;

use starknet::signers::SigningKey;

use crate::config::CloudEnv;

use aws::PragmaSignerBuilder as AwsPragmaSignerBuilder;
use gcp::PragmaSignerBuilder as GcpPragmaSignerBuilder;

pub async fn build_signer(cloud_env: CloudEnv, is_production: bool) -> Option<SigningKey> {
    if is_production {
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
    }
}
