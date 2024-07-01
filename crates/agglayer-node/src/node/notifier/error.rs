#[derive(Debug, thiserror::Error)]
pub(crate) enum NotifierError {
    #[error("unable to build notifier")]
    UnableToBuildNotifier,
}
