use std::env;
use std::str::FromStr;

use agglayer_config::certificate_orchestrator::prover::ProverConfig;
use ethers::signers::LocalWallet;
use sp1_sdk::{network::client::NetworkClient, LocalProver};

use crate::node::notifier::AggregatorNotifier;

mod sp1 {
    use agglayer_certificate_orchestrator::EpochPacker;

    use super::*;
    #[tokio::test]
    async fn aggregator_notifier_for_network() {
        println!("{:?}", env::var("SP1_PRIVATE_KEY"));
        let config = ProverConfig::SP1Network {};
        let notifier = AggregatorNotifier::try_from(config);
        assert!(notifier.is_ok());

        let notifier = notifier.unwrap();

        let result = notifier.pack(1, vec![]).unwrap().await;
    }
}
