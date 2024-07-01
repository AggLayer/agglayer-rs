use std::{num::NonZeroU64, sync::Arc};

use agglayer_certificate_orchestrator::CertificateOrchestrator;
use agglayer_clock::{Clock, TimeClock};
use agglayer_config::{Config, Epoch};
use agglayer_signer::ConfiguredSigner;
use anyhow::Result;
use ethers::{
    middleware::MiddlewareBuilder as _,
    providers::{Http, Provider},
};
use tokio::{join, sync::mpsc, task::JoinHandle};
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use tracing::debug;

use self::notifier::AggregatorNotifier;
use crate::{kernel::Kernel, rpc::AgglayerImpl};

mod notifier;

pub(crate) struct Node {
    rpc_handle: JoinHandle<()>,
    certificate_orchestrator_handle: JoinHandle<()>,
}

#[buildstructor::buildstructor]
impl Node {
    /// Function that setups and starts the Agglayer node.
    ///
    /// The available methods are:
    ///
    /// - `builder`: Creates a new builder instance.
    /// - `config`: Sets the configuration.
    /// - `start`: Starts the Agglayer node.
    ///
    /// # Examples
    /// ```no_compile
    /// # use std::sync::Arc;
    /// # use agglayer_config::Config;
    /// # use agglayer_node::Node;
    /// # use tokio_util::sync::CancellationToken;
    /// # use anyhow::Result;
    /// #
    /// async fn start_node() -> Result<()> {
    ///    let config: Arc<Config> = Arc::new(Config::default());
    ///
    ///    Node::builder()
    ///      .config(config)
    ///      .cancellation_token(CancellationToken::new())
    ///      .start()
    ///      .await?;
    ///
    ///    Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The L1 node URL is invalid.
    /// - The configured signer is invalid.
    /// - The RPC server failed to start.
    /// - The [`TimeClock`] failed to start.
    #[builder(entry = "builder", exit = "start", visibility = "pub(crate)")]
    pub(crate) async fn start(
        config: Arc<Config>,
        cancellation_token: CancellationToken,
    ) -> Result<Self> {
        // Create a new L1 RPC provider with the configured signer.
        let rpc = Provider::<Http>::try_from(config.l1.node_url.as_str())?
            .with_signer(ConfiguredSigner::new(config.clone()).await?);

        // Construct the core.
        let core = Kernel::new(rpc, config.clone());

        // Spawn the TimeClock.
        let clock_ref = match &config.epoch {
            Epoch::TimeClock(cfg) => {
                let duration =
                    NonZeroU64::new(cfg.epoch_duration.as_secs()).ok_or(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "EpochDuration is invalid",
                    ))?;
                let clock = TimeClock::new_now(duration);

                clock.spawn(cancellation_token.clone()).await?
            }
        };

        let aggregator_task: AggregatorNotifier =
            config.certificate_orchestrator.prover.clone().try_into()?;
        let clock_subscription =
            tokio_stream::wrappers::BroadcastStream::new(clock_ref.subscribe()?)
                .filter_map(|value| value.ok());

        let (data_sender, data_receiver) = mpsc::channel(
            config
                .certificate_orchestrator
                .input_backpressure_buffer_size,
        );

        let certificate_orchestrator_handle = CertificateOrchestrator::builder()
            .clock(clock_subscription)
            .data_receiver(data_receiver)
            .cancellation_token(cancellation_token.clone())
            .epoch_packing_task_builder(aggregator_task)
            .start()
            .await?;

        // Bind the core to the RPC server.
        let server_handle = AgglayerImpl::new(core, data_sender).start(config).await?;

        let rpc_handle = tokio::spawn(async move {
            tokio::select! {
                _ = server_handle.stopped() => {},
                _ = cancellation_token.cancelled() => {
                    debug!("Node RPC shutdown requested.");
                }
            }
        });

        let node = Self {
            rpc_handle,
            certificate_orchestrator_handle,
        };

        Ok(node)
    }

    pub(crate) async fn await_shutdown(self) {
        debug!("Node shutdown started.");
        _ = join!(self.rpc_handle, self.certificate_orchestrator_handle);
        debug!("Node shutdown completed.");
    }
}
