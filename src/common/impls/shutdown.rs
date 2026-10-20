use crate::common::structs::shutdown::Shutdown;

impl Shutdown {
    /// Starts listening for Ctrl-C and, on Unix, SIGTERM (what `docker stop` and `systemctl stop`
    /// send). Must be called inside the tokio runtime.
    pub fn listen() -> Self {
        let (tx, rx) = tokio::sync::watch::channel(false);
        tokio::spawn(async move {
            #[cfg(unix)]
            {
                let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("Unable to install the SIGTERM handler");
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = terminate.recv() => {}
                }
            }
            #[cfg(not(unix))]
            let _ = tokio::signal::ctrl_c().await;
            let _ = tx.send(true);
        });
        Self(rx)
    }

    /// Completes once shutdown has been signalled.
    pub async fn handle(&self) {
        let _ = self.0.clone().wait_for(|stopped| *stopped).await;
    }
}
