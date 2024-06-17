use std::sync::Arc;

use agglayer_certificate_orchestrator::{EpochPacker, Error};
use futures::future::BoxFuture;
use sp1_sdk::{NetworkProver, Prover, SP1Proof, SP1ProvingKey, SP1Stdin, SP1VerifyingKey};
use tracing::{debug, error, info};

const ELF: &[u8] = include_bytes!("../../../../elf/riscv32im-succinct-zkvm-elf");

/// The SP1 prover that can generate and verify proofs for the aggregator.
pub(crate) struct SP1 {
    prover: NetworkProver,
    proving_key: SP1ProvingKey,
    verifying_key: SP1VerifyingKey,
}

impl AggregatorProver for SP1 {
    type Proof = SP1Proof;

    fn prove(
        &self,
        to_pack: Vec<()>,
    ) -> impl std::future::Future<Output = Result<Self::Proof, anyhow::Error>> + std::marker::Send
    {
        let mut stdin = SP1Stdin::new();
        stdin.write(&to_pack);

        async move { self.prover.prove(&self.proving_key.elf, stdin).await }
    }

    fn verify(&self, proof: &Self::Proof) -> Result<(), anyhow::Error> {
        Ok(self.prover.verify(proof, &self.verifying_key)?)
    }
}

pub(crate) trait AggregatorProver: Send + Sync {
    type Proof;

    fn prove(
        &self,
        to_pack: Vec<()>,
    ) -> impl std::future::Future<Output = Result<Self::Proof, anyhow::Error>> + std::marker::Send;
    fn verify(&self, proof: &Self::Proof) -> Result<(), anyhow::Error>;
}

#[derive(Clone)]
pub(crate) struct AggregatorNotifier<P> {
    prover: Arc<P>,
}

impl AggregatorNotifier<SP1> {
    pub(crate) fn new() -> Self {
        let client = NetworkProver::new();
        let (proving_key, verifying_key) = client.setup(ELF);

        let prover = SP1 {
            prover: client,
            proving_key,
            verifying_key,
        };

        Self {
            prover: Arc::new(prover),
        }
    }
}

impl<P> EpochPacker for AggregatorNotifier<P>
where
    P: AggregatorProver + Send + 'static,
{
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
                info!("successfully generated and verified proof for the program!");
                Ok(())
            }
        }))
    }
}
