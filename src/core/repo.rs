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
    fn commit_tx(&self, tx: Self::Tx) -> impl Future<Output = Result<(), ChonkitError>>;

    /// Abort a database transaction.
    fn abort_tx(&self, tx: Self::Tx) -> impl Future<Output = Result<(), ChonkitError>>;
}

/// Uses `$repo` to start a transaction, passing it to the provided `$op`.
/// The provided `$op` must return a result.
/// Aborts the transaction on error and commits on success.
#[macro_export]
macro_rules! transaction {
    ($repo:expr, $op:expr) => {{
        let mut tx = $repo.start_tx().await?;
        let result = { $op(&mut tx) }.await;
        match result {
            Ok(out) => {
                $repo.commit_tx(tx).await?;
                Result::<_, ChonkitError>::Ok(out)
            }
            Err(err) => {
                $repo.abort_tx(tx).await?;
                return Err(err);
            }
        }
    }};
}
