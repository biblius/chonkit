use crate::error::ChonkitError;
use std::future::Future;

pub mod document;
pub mod vector;

/// Bound for repositories that support atomic operations.
pub trait Atomic {
    /// Transaction type.
    type Tx;

    /// Start a database transaction.
    fn start_tx(&self) -> impl Future<Output = Result<Self::Tx, ChonkitError>>;

    /// Commit a database transaction.
    fn commit_tx(tx: Self::Tx) -> impl Future<Output = Result<(), ChonkitError>>;

    /// Abort a database transaction.
    fn abort_tx(tx: Self::Tx) -> impl Future<Output = Result<(), ChonkitError>>;
}

/// Helper for executing started transactions.
/// Aborts the transaction on error and commits on success.
#[macro_export]
macro_rules! transaction {
    ($self:ident, $tx:ident, $op:expr) => {
        async {
            let result = { $op }.await;
            match result {
                Ok(out) => {
                    <R as Atomic>::commit_tx($tx).await?;
                    Ok(out)
                }
                Err(err) => {
                    <R as Atomic>::abort_tx($tx).await?;
                    return Err(err);
                }
            }
        }
    };
}
