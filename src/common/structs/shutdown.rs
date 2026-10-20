/// Shutdown signal shared by every long-running task: [`handle`](Shutdown::handle) completes once
/// Ctrl-C or, on Unix, SIGTERM has arrived. Cheap to clone. Listening starts in
/// [`listen`](Shutdown::listen), so a signal is not lost before anyone waits on it.
#[derive(Clone)]
pub struct Shutdown(pub(crate) tokio::sync::watch::Receiver<bool>);
