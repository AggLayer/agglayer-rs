use crate::{
    bridge_exit::NetworkId, certificate::Certificate, global_state::LocalNetworkState,
    keccak::Digest,
};

/// Represents all errors that can occur while generating the proof.
#[derive(Debug)]
pub enum ProofError {
    InvalidLocalExitRoot { got: Digest, expected: Digest },
    HasDebt { network: NetworkId },
}

pub type ExitRoot = Digest;
pub type BalanceRoot = Digest;
pub type LeafProofOutput = (ExitRoot, BalanceRoot);

/// Proves that the given [`Certificate`] can be applied on the given [`LocalNetworkState`].
pub fn generate_leaf_proof(
    initial_network_state: LocalNetworkState,
    certificate: Certificate,
) -> Result<LeafProofOutput, ProofError> {
    let mut network_state = initial_network_state;

    let (new_exit_root, new_balance_root) = network_state.apply_certificate(certificate)?;

    Ok((new_exit_root, new_balance_root))
}
