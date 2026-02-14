use sentry::{
    Transaction,
    TransactionContext
};

pub fn start_trace_transaction(name: &str, operation: &str) -> Option<Transaction> {
    if log::max_level() >= log::LevelFilter::Trace {
        let ctx = TransactionContext::new(name, operation);
        Some(sentry::start_transaction(ctx))
    } else {
        None
    }
}

#[macro_export]
macro_rules! instrument_with_sentry {
    (name = $name:expr, op = $op:expr, $body:block) => {{
        let transaction = $crate::utils::sentry_tracing::start_trace_transaction($name, $op);
        let result = $body;
        if let Some(txn) = transaction {
            txn.finish();
        }
        result
    }};
}