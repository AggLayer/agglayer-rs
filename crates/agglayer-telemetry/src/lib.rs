use std::net::SocketAddr;

use axum::{
    extract::State,
    http::{Response, StatusCode},
    routing::get,
    serve::Serve,
    Router,
};
use lazy_static::lazy_static;
use opentelemetry::global;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use prometheus::{Encoder as _, Registry, TextEncoder};
use tracing::info;

use crate::{
    constant::{AGGLAYER_KERNEL_OTEL_SCOPE_NAME, AGGLAYER_RPC_OTEL_SCOPE_NAME},
    error::MetricsError,
};

mod constant;
mod error;

pub use error::Error;
pub use opentelemetry::KeyValue;

lazy_static! {
    // Backward compatibility with the old metrics from agglayer go implementation
    // Those metrics are not linked to any registry
    pub static ref SEND_TX: opentelemetry::metrics::Counter<u64> = global::meter(AGGLAYER_RPC_OTEL_SCOPE_NAME)
        .u64_counter("send_tx")
        .with_description("Number of transactions received on the RPC")
        .init();

    pub static ref VERIFY_ZKP: opentelemetry::metrics::Counter<u64> = global::meter(AGGLAYER_KERNEL_OTEL_SCOPE_NAME)
        .u64_counter("verify_zkp")
        .with_description("Number of ZKP verifications")
        .init();

    pub static ref VERIFY_SIGNATURE: opentelemetry::metrics::Counter<u64> = global::meter(AGGLAYER_KERNEL_OTEL_SCOPE_NAME)
        .u64_counter("verify_signature")
        .with_description("Number of signature verifications")
        .init();

    pub static ref CHECK_TX: opentelemetry::metrics::Counter<u64> = global::meter(AGGLAYER_KERNEL_OTEL_SCOPE_NAME)
        .u64_counter("check_tx")
        .with_description("Number of transactions checked")
        .init();

    pub static ref EXECUTE: opentelemetry::metrics::Counter<u64> = global::meter(AGGLAYER_KERNEL_OTEL_SCOPE_NAME)
        .u64_counter("execute")
        .with_description("Number of transactions executed")
        .init();

    pub static ref SETTLE: opentelemetry::metrics::Counter<u64> = global::meter(AGGLAYER_KERNEL_OTEL_SCOPE_NAME)
        .u64_counter("settle")
        .with_description("Number of transactions settled")
        .init();
}

pub struct ServerBuilder {}

#[buildstructor::buildstructor]
impl ServerBuilder {
    /// Function that builds a new Metrics server and returns a [`Serve`]
    /// instance ready to be spawn.
    ///
    /// The available methods are:
    ///
    /// - `builder`: Creates a new builder instance.
    /// - `addr`: Sets the [`SocketAddr`] to bind the metrics server to.
    /// - `registry`: Sets the [`Registry`] to use for metrics. (optional)
    /// - `build`: Builds the metrics server and returns a [`Serve`] instance.
    ///
    /// # Examples
    /// ```no_run
    /// # use std::sync::Arc;
    /// # use agglayer_config::Config;
    /// # use agglayer_node::Node;
    /// # use anyhow::Result;
    /// #
    /// use axum::serve::Serve;
    ///
    /// async fn build_metrics() -> Result<Serve, Error> {
    ///    ServerBuilder::builder()
    ///      .addr("127.0.0.1".parse().unwrap())
    ///      .build()
    ///      .await?;
    ///
    ///    Ok(())
    /// }
    /// ```
    ///
    ///
    /// # Panics
    ///
    /// Panics on failure of the gather_metrics internal methods (unlikely)
    ///
    /// # Errors
    ///
    /// This function will return an error if the provided addr is invalid
    #[builder(entry = "builder", exit = "build", visibility = "pub")]
    pub async fn serve(
        addr: SocketAddr,
        registry: Option<Registry>,
    ) -> Result<Serve<axum::routing::IntoMakeService<Router>, axum::Router>, Error> {
        let registry = registry.unwrap_or_default();
        let _ = Self::init_meter_provider(&registry);

        let app = Router::new()
            .route(
                "/metrics",
                get(|State(registry): State<prometheus::Registry>| async move {
                    match Self::gather_metrics(&registry) {
                        Ok(metrics) => Response::new(metrics),
                        Err(error) => Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(error.to_string())
                            .unwrap(),
                    }
                }),
            )
            .with_state(registry);

        info!("Starting metrics server on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        Ok(axum::serve(listener, app.into_make_service()))
    }

    fn init_meter_provider(registry: &Registry) -> Result<(), MetricsError> {
        // configure OpenTelemetry to use the registry
        let exporter = opentelemetry_prometheus::exporter()
            .with_registry(registry.clone())
            .build()?;

        // set up a meter meter to create instruments
        let provider = SdkMeterProvider::builder().with_reader(exporter).build();

        global::set_meter_provider(provider);
        Ok(())
    }

    fn gather_metrics(registry: &prometheus::Registry) -> Result<String, MetricsError> {
        // Encode data as text or protobuf
        let encoder = TextEncoder::new();
        let metric_families = registry.gather();
        let mut result = Vec::new();
        encoder.encode(&metric_families, &mut result)?;

        Ok(String::from_utf8(result)?)
    }
}
