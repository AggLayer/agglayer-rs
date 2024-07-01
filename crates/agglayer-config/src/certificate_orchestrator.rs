use prover::ProverConfig;
use serde::Deserialize;

pub mod prover;

/// The CertificateOrchestrator configuration.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct CertificateOrchestrator {
    #[serde(default = "default_input_backpressure_buffer_size_default")]
    pub input_backpressure_buffer_size: usize,

    #[serde(default = "default_prover_config_default")]
    pub prover: ProverConfig,
}

impl Default for CertificateOrchestrator {
    fn default() -> Self {
        Self {
            input_backpressure_buffer_size: default_input_backpressure_buffer_size_default(),
            prover: default_prover_config_default(),
        }
    }
}

fn default_input_backpressure_buffer_size_default() -> usize {
    1_000
}

fn default_prover_config_default() -> ProverConfig {
    ProverConfig::SP1Local {}
}
