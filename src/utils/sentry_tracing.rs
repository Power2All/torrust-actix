use sentry::{
    Transaction,
    TransactionContext
};

/// Starts a Sentry performance transaction when trace-level logging is enabled; returns
/// `None` otherwise so instrumentation is free in production.
pub fn start_trace_transaction(name: &str, operation: &str) -> Option<Transaction> {
    if log::max_level() >= log::LevelFilter::Trace {
        let ctx = TransactionContext::new(name, operation);
        Some(sentry::start_transaction(ctx))
    } else {
        None
    }
}
