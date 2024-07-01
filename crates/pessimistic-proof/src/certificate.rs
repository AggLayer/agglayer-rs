use serde::{Deserialize, Serialize};

use crate::{bridge_exit::NetworkId, keccak::Digest, BridgeExit};

/// Represents the data submitted by the CDKs to the AggLayer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Certificate {
    /// Origin network which emitted this certificate
    pub origin_network: NetworkId,
    /// Initial local exit root
    pub prev_local_exit_root: Digest,
    /// Set of bridge exits
    pub bridge_exits: Vec<BridgeExit>,
    /// Set of imported bridge exits
    pub imported_bridge_exits: Vec<BridgeExit>,
    /// Signature that commits to the state transition
    pub signature: (),
}

impl Certificate {
    /// Creates a new [`Certificate`].
    pub fn new(
        origin_network: NetworkId,
        prev_local_exit_root: Digest,
        bridge_exits: Vec<BridgeExit>,
    ) -> Self {
        Self {
            origin_network,
            prev_local_exit_root,
            bridge_exits,
            imported_bridge_exits: Default::default(),
            signature: (),
        }
    }
}
