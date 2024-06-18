use std::sync::Arc;

use agglayer_certificate_orchestrator::{EpochPacker, Error};
use agglayer_config::certificate_orchestrator::prover::ProverConfig;
use error::NotifierError;
use futures::future::BoxFuture;
use proof::Proof;
use sp1::SP1;
use sp1_sdk::{LocalProver, NetworkProver};
use tracing::{debug, error, info};

const ELF: &[u8] =
    include_bytes!("../../../pessimistic-proof-program/elf/riscv32im-succinct-zkvm-elf");

mod error;
mod proof;
mod sp1;
#[cfg(test)]
mod tests;

pub(crate) trait AggregatorProver: Send + Sync {
    fn prove(&self, to_pack: Vec<()>) -> BoxFuture<'_, Result<Proof, anyhow::Error>>;
    fn verify(&self, proof: &Proof) -> Result<(), anyhow::Error>;
}

#[derive(Clone)]
pub(crate) struct AggregatorNotifier {
    prover: Arc<dyn AggregatorProver>,
}

impl TryFrom<ProverConfig> for AggregatorNotifier {
    type Error = NotifierError;

    fn try_from(config: ProverConfig) -> Result<Self, Self::Error> {
        match config {
            ProverConfig::SP1Network {} => Ok(Self {
                prover: Arc::new(SP1::new(NetworkProver::new(), ELF)),
            }),
            ProverConfig::SP1Local {} => Ok(Self {
                prover: Arc::new(SP1::new(LocalProver::new(), ELF)),
            }),
            ProverConfig::SP1Mock {} => Err(NotifierError::UnableToBuildNotifier),
        }
    }
}

impl EpochPacker for AggregatorNotifier {
    fn pack<T: IntoIterator<Item = ()>>(
        &self,
        epoch: u64,
        to_pack: T,
    ) -> Result<BoxFuture<Result<(), Error>>, Error> {
        // TODO: Implement the aggregator notifier.
        let to_pack = to_pack.into_iter().collect::<Vec<_>>();

        debug!(
            "Start packing epoch {} with {} certificates",
            epoch,
            to_pack.len()
        );

        let proving_request = self.prover.prove(to_pack);

        Ok(Box::pin(async move {
            let proof = proving_request.await.unwrap();

            if let Err(error) = self.prover.verify(&proof) {
                error!("failed to verify proof: {:?}", error);

                Err(Error::ProofVerificationFailed)
            } else {
                info!(
                    "successfully generated and verified proof for the
            program!"
                );
                Ok(())
            }
        }))
    }
}
