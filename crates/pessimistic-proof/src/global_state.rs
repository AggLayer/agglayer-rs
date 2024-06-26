use std::collections::BTreeSet;

use crate::{
    certificate::Certificate,
    local_balance_tree::BalanceTree,
    local_exit_tree::{hasher::Keccak256Hasher, LocalExitTree},
    proof::{BalanceRoot, ExitRoot},
    BridgeExit, NetworkId, ProofError,
};

/// Local state of one network.
/// The AggLayer tracks the [`LocalNetworkState`] for all networks.
/// Eventually, this state will be entirely tracked by the networks themselves.
pub struct LocalNetworkState {
    /// Commitment to the [`BridgeExit`].
    exit_tree: LocalExitTree<Keccak256Hasher>,
    /// Commitment to the balance for each token.
    balance_tree: BalanceTree,
    /// Commitment to the imported [`BridgeExit`].
    #[allow(dead_code)]
    nullifier_tree: BTreeSet<(NetworkId, BridgeExit)>,
}

impl LocalNetworkState {
    /// Apply the [`Certificate`] on the current [`State`].
    /// Returns the commitment on the resulting state if successful.
    pub fn apply_certificate(
        &mut self,
        certificate: Certificate,
    ) -> Result<(ExitRoot, BalanceRoot), ProofError> {
        // Apply on Exit Tree
        {
            let computed_root = self.exit_tree.get_root();
            if computed_root != certificate.prev_local_exit_root {
                return Err(ProofError::InvalidLocalExitRoot {
                    got: computed_root,
                    expected: certificate.prev_local_exit_root,
                });
            }

            for bridge_exit in &certificate.bridge_exits {
                self.exit_tree.add_leaf(bridge_exit.hash());
            }
        }

        // Apply on Balance Tree
        {
            for bridge_exit in certificate.bridge_exits {
                self.balance_tree.withdraw(bridge_exit.token_info, bridge_exit.amount);
            }

            // Check whether the sender has some debt
            if self.balance_tree.has_debt() {
                return Err(ProofError::HasDebt {
                    network: certificate.origin_network,
                });
            }
        };

        // TODO: Apply on the nullifier tree

        Ok((self.exit_tree.get_root(), self.balance_tree.hash()))
    }
}
