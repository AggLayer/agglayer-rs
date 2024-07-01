use std::{
    collections::{BTreeMap, VecDeque},
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use agglayer_clock::Event;
use futures_util::{future::BoxFuture, Stream, StreamExt};
use tokio::{
    sync::mpsc::Receiver,
    task::{JoinHandle, JoinSet},
};
use tokio_util::sync::{CancellationToken, WaitForCancellationFutureOwned};
use tracing::{debug, error};

#[cfg(test)]
mod tests;

const MAX_POLL_READS: usize = 1_000;

/// Certificate orchestrator that receives certificates from CDKs.
/// It collects certificates and sends them to the epoch packer when an epoch
/// ends.
pub struct CertificateOrchestrator<C, A> {
    /// Epoch packing task resolver.
    epoch_packing_tasks: JoinSet<Result<(), Error>>,
    /// Epoch packing task builder.
    epoch_packing_task_builder: Arc<A>,
    /// Clock stream to receive EpochEnded events.
    clock: C,
    /// Certificates received from CDKs.
    received_certificates: VecDeque<()>,
    /// Certificates to pack for each epoch.
    pub(crate) to_pack: BTreeMap<u64, VecDeque<()>>,
    /// Receiver for certificates coming from CDKs.
    data_receiver: Receiver<()>,
    /// Cancellation token for graceful shutdown.
    cancellation_token: Pin<Box<WaitForCancellationFutureOwned>>,
}

impl<C, A> CertificateOrchestrator<C, A> {
    /// Creates a new CertificateOrchestrator instance.
    pub(crate) fn new(
        clock: C,
        data_receiver: Receiver<()>,
        cancellation_token: CancellationToken,
        epoch_packing_task_builder: A,
    ) -> Self {
        Self {
            epoch_packing_tasks: JoinSet::new(),
            clock,
            epoch_packing_task_builder: Arc::new(epoch_packing_task_builder),
            data_receiver,
            received_certificates: VecDeque::new(),
            to_pack: BTreeMap::default(),
            cancellation_token: Box::pin(cancellation_token.cancelled_owned()),
        }
    }
}

#[buildstructor::buildstructor]
impl<C, A> CertificateOrchestrator<C, A>
where
    C: Stream<Item = Event> + Unpin + Send + 'static,
    A: EpochPacker + Send + Sync,
{
    /// Function that setups and starts the CertificateOrchestrator.
    ///
    /// The available methods are:
    ///
    /// - `builder`: Creates a new builder instance.
    /// - `clock`: Sets clock stream to receive EpochEnded events.
    /// - `data_receiver`: Sets the receiver for certificates coming from CDKs.
    /// - `cancellation_token`: Sets the cancellation token for graceful
    ///   shutdown.
    /// - `epoch_packing_builder`: Sets the task builder for epoch packing.
    /// - `start`: Starts the CertificateOrchestrator.
    ///
    /// # Examples
    /// ```
    /// # use agglayer_certificate_orchestrator::Error;
    /// # use agglayer_certificate_orchestrator::EpochPacker;
    /// # use agglayer_certificate_orchestrator::CertificateOrchestrator;
    /// # use tokio_stream::wrappers::BroadcastStream;
    /// # use tokio_util::sync::CancellationToken;
    /// # use futures_util::future::BoxFuture;
    /// # use tokio_stream::StreamExt;
    ///
    /// ##[derive(Clone)]
    /// pub struct AggregatorNotifier {}
    ///
    /// impl AggregatorNotifier {
    ///     pub(crate) fn new() -> Self {
    ///         Self {}
    ///     }
    /// }
    ///
    /// impl EpochPacker for AggregatorNotifier {
    ///     fn pack<T: IntoIterator<Item = ()>>(
    ///         &self,
    ///         epoch: u64,
    ///         to_pack: T,
    ///     ) -> Result<BoxFuture<Result<(), Error>>, Error> {Ok(Box::pin(async move {Ok(())}))}
    /// }
    ///
    /// async fn start() -> Result<(), ()> {
    ///    let (sender, receiver) = tokio::sync::broadcast::channel(1);
    ///    let clock_stream = BroadcastStream::new(sender.subscribe()).filter_map(|value| value.ok());
    ///    let notifier = AggregatorNotifier::new();
    ///    let data_receiver = tokio::sync::mpsc::channel(1).1;
    ///
    ///    CertificateOrchestrator::builder()
    ///      .clock(clock_stream)
    ///      .data_receiver(data_receiver)
    ///      .cancellation_token(CancellationToken::new())
    ///      .epoch_packing_task_builder(notifier)
    ///      .start()
    ///      .await
    ///      .unwrap();
    ///
    ///    Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This function can't fail but returns a Result for convenience and future
    ///
    /// evolution.
    #[builder(entry = "builder", exit = "start", visibility = "pub")]
    pub async fn start(
        clock: C,
        data_receiver: Receiver<()>,
        cancellation_token: CancellationToken,
        epoch_packing_task_builder: A,
    ) -> anyhow::Result<JoinHandle<()>> {
        let orchestrator = Self::new(
            clock,
            data_receiver,
            cancellation_token,
            epoch_packing_task_builder,
        );

        let handle = tokio::spawn(orchestrator);

        Ok(handle)
    }
}

impl<C, A> Future for CertificateOrchestrator<C, A>
where
    C: Stream<Item = Event> + Send + Unpin + 'static,
    A: EpochPacker + Send + Sync,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Check if the orchestrator has been cancelled and should shutdown.
        if self.cancellation_token.as_mut().poll(cx).is_ready() {
            debug!("Certificate orchestrator cancelled by token");

            return Poll::Ready(());
        }

        // Poll the notification tasks to check if any have errored.
        match self.epoch_packing_tasks.poll_join_next(cx) {
            Poll::Ready(Some(Ok(Err(error)))) => {
                error!("Error during epoch packing: {:?}", error)
            }
            Poll::Ready(Some(Err(error))) => {
                error!("Critical error during epoch packing: {:?}", error);
            }
            _ => {}
        }

        if let Some((epoch, certificates)) = self.to_pack.pop_first() {
            debug!("Packing certificates for epoch {}", epoch);
            // Create a new task to pack the certificates for this epoch
            let task = self.epoch_packing_task_builder.clone();

            self.epoch_packing_tasks
                .spawn(async move { task.pack(epoch, certificates)?.await });
        }

        let mut received = vec![];
        if let Poll::Ready(1usize..) =
            self.data_receiver
                .poll_recv_many(cx, &mut received, MAX_POLL_READS)
        {
            self.received_certificates.extend(received);

            return self.poll(cx);
        }

        if let Poll::Ready(Some(Event::EpochEnded(epoch))) = self.clock.poll_next_unpin(cx) {
            debug!("Epoch change event received: {}", epoch);

            let to_pack = std::mem::take(&mut self.received_certificates);
            self.to_pack.insert(epoch, to_pack);

            return self.poll(cx);
        }

        Poll::Pending
    }
}

pub trait EpochPacker: Unpin + Send + 'static {
    fn pack<T: IntoIterator<Item = ()>>(
        &self,
        epoch: u64,
        to_pack: T,
    ) -> Result<BoxFuture<Result<(), Error>>, Error>;
}

#[derive(Debug)]
pub enum Error {
    ProofVerificationFailed,
}
