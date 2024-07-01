//! Agglayer configuration.
//!
//! The agglayer is configured via its TOML configuration file, `agglayer.toml`
//! by default, which is deserialized into the [`Config`] struct.

use std::collections::HashMap;

use auth::deserialize_auth;
use outbound::OutboundConfig;
use serde::Deserialize;
use shutdown::ShutdownConfig;
use url::Url;

use self::{rpc::deserialize_rpc_map, telemetry::TelemetryConfig};

pub(crate) const DEFAULT_IP: std::net::Ipv4Addr = std::net::Ipv4Addr::new(0, 0, 0, 0);

pub(crate) mod auth;
pub mod certificate_orchestrator;
pub(crate) mod epoch;
pub(crate) mod l1;
pub mod log;
pub(crate) mod outbound;
pub(crate) mod rpc;
pub mod shutdown;
pub(crate) mod telemetry;

pub use auth::{AuthConfig, GcpKmsConfig, LocalConfig, PrivateKey};
pub use epoch::Epoch;
pub use l1::L1;
pub use log::Log;
pub use rpc::RpcConfig;

/// The Agglayer configuration.
#[derive(Deserialize, Debug)]
#[cfg_attr(any(test, feature = "testutils"), derive(Default))]
pub struct Config {
    /// A map of Zkevm node RPC endpoints for each rollup.
    ///
    /// The key is the rollup ID, and the value is the URL of the associated RPC
    /// endpoint.
    #[serde(rename = "FullNodeRPCs", deserialize_with = "deserialize_rpc_map")]
    pub full_node_rpcs: HashMap<u32, Url>,
    /// The log configuration.
    #[serde(rename = "Log")]
    pub log: Log,
    /// The local RPC server configuration.
    #[serde(rename = "RPC")]
    pub rpc: RpcConfig,
    /// The configuration for every outbound network component.
    #[serde(default)]
    pub outbound: OutboundConfig,
    /// The L1 configuration.
    #[serde(rename = "L1")]
    pub l1: L1,
    /// The authentication configuration.
    #[serde(alias = "EthTxManager", default, deserialize_with = "deserialize_auth")]
    pub auth: AuthConfig,
    /// Telemetry configuration.
    #[serde(rename = "Telemetry")]
    pub telemetry: TelemetryConfig,

    /// The Epoch configuration.
    #[serde(rename = "Epoch", default = "Epoch::default")]
    pub epoch: Epoch,

    /// The list of configuration options used during shutdown.
    #[serde(default)]
    pub shutdown: ShutdownConfig,

    /// The certificate orchestrator configuration.
    #[serde(rename = "CertificateOrchestrator", default)]
    pub certificate_orchestrator: certificate_orchestrator::CertificateOrchestrator,
}

impl Config {
    /// Get the target RPC socket address from the configuration.
    pub fn rpc_addr(&self) -> std::net::SocketAddr {
        std::net::SocketAddr::from((self.rpc.host, self.rpc.port))
    }
}
