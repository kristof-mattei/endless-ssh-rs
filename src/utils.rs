pub mod env;
pub mod url;

use color_eyre::eyre;
use tokio::task::JoinHandle;

/// Use this when you have a `JoinHandle<Result<T, E>>`
/// and you want to use it with `tokio::try_join!`
/// when the task completes with an `Result::Err`
/// the `JoinHandle` itself will be `Result::Ok` and thus not
/// trigger the `tokio::try_join!`. This function flattens the 2:
/// `Result::Ok(T)` when both the join-handle AND
/// the result of the inner function are `Result::Ok`, and `Result::Err`
/// when either the join failed, or the inner task failed
///
/// # Errors
/// * When there is an issue executing the task
/// * When the task itself failed
#[expect(unused, reason = "Library code")]
pub async fn flatten_handle<T, E>(handle: JoinHandle<Result<T, E>>) -> Result<T, eyre::Report>
where
    E: 'static + Sync + Send,
    eyre::Report: From<E>,
{
    match handle.await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => Err(err.into()),
        Err(err) => Err(err.into()),
    }
}
