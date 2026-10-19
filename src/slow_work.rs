//! Work too slow for the threads that answer requests (#370, #373).
//!
//! An Operation is answered on the Tokio worker that took its request, and a
//! worker carries many requests at once. Hashing a password, decoding a
//! picture or waiting on a PDF's reader holds a worker for as long as it
//! takes, and every other request on that worker waits with it. This moves
//! such work aside.

/// Run slow work without holding a request worker. On a worker it tells the
/// runtime this worker is busy, and its other requests go elsewhere. Outside
/// a runtime that can do that (a test, the terminal, a Job on the blocking
/// pool) it simply runs.
pub fn off_the_request_workers<T>(work: impl FnOnce() -> T) -> T {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(work)
        }
        _ => work(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// While one request waits on slow work, another on the same and only
    /// worker is answered.
    #[test]
    fn slow_work_holds_no_request_worker() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let (said, heard) = std::sync::mpsc::channel::<()>();
            let slow = tokio::spawn(async move {
                // Answered only once the other request has been.
                off_the_request_workers(|| heard.recv_timeout(Duration::from_secs(10)).is_ok())
            });
            let other = tokio::spawn(async move {
                let _ = said.send(());
            });
            tokio::time::timeout(Duration::from_secs(5), other)
                .await
                .expect("the other request waited on the slow work")
                .unwrap();
            assert!(slow.await.unwrap());
        });
    }
}
