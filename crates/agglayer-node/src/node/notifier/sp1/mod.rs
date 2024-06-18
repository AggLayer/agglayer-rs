use std::sync::Arc;

use futures::{future::BoxFuture, FutureExt as _};
use sp1_sdk::{LocalProver, NetworkProver, Prover as _, SP1ProvingKey, SP1Stdin, SP1VerifyingKey};
use tokio::task::spawn_blocking;

use super::Proof;

pub(crate) struct SP1<P> {
    prover: Arc<P>,
    proving_key: Arc<SP1ProvingKey>,
    verifying_key: SP1VerifyingKey,
}

impl<P: sp1_sdk::Prover> SP1<P> {
    pub(super) fn new(prover: P, elf: &[u8]) -> Self {
        let (proving_key, verifying_key) = prover.setup(elf);

        Self {
            prover: Arc::new(prover),
            proving_key: Arc::new(proving_key),
            verifying_key,
        }
    }
}

impl super::AggregatorProver for SP1<LocalProver> {
    fn prove(&self, to_pack: Vec<()>) -> BoxFuture<'_, Result<Proof, anyhow::Error>> {
        let mut stdin = SP1Stdin::new();
        stdin.write(&to_pack);

        let proving_key = self.proving_key.clone();
        let prover = self.prover.clone();

        async move {
            spawn_blocking(move || prover.prove(&proving_key, stdin))
                .await?
                .map(Proof::SP1)
        }
        .boxed()
    }

    fn verify(&self, proof: &Proof) -> Result<(), anyhow::Error> {
        let Proof::SP1(proof) = proof;

        Ok(self.prover.verify(proof, &self.verifying_key)?)
    }
}

impl super::AggregatorProver for SP1<NetworkProver> {
    fn prove(&self, to_pack: Vec<()>) -> BoxFuture<'_, Result<Proof, anyhow::Error>> {
        let mut stdin = SP1Stdin::new();
        stdin.write(&to_pack);

        let proving_key = self.proving_key.clone();
        let prover = self.prover.clone();

        async move { prover.prove(&proving_key.elf, stdin).await.map(Proof::SP1) }.boxed()
    }

    fn verify(&self, proof: &Proof) -> Result<(), anyhow::Error> {
        let Proof::SP1(proof) = proof;
        Ok(self.prover.verify(proof, &self.verifying_key)?)
    }
}
