use crate::HealthState;
use std::sync::Arc;
use tokio::sync::{mpsc, watch, Semaphore};

// =========================================================================
// Tokio Sync Primitives (P1 #18)
// =========================================================================
//
// Demonstrates watch, mpsc, and Semaphore — the three most important
// concurrency primitives beyond Mutex/RwLock.

/// One-writer, many-reader health state broadcaster using `tokio::sync::watch`.
///
/// The health engine writes state updates; any number of subscribers can
/// read the current state without blocking the writer.
///
/// # Examples
/// ```ignore
/// let broadcaster = HealthBroadcaster::new(HealthState::Green);
/// let mut rx = broadcaster.subscribe();
///
/// // Writer updates:
/// broadcaster.update(HealthState::Yellow);
///
/// // Readers see the change:
/// assert_eq!(*rx.borrow(), HealthState::Yellow);
/// ```
pub struct HealthBroadcaster {
    tx: watch::Sender<HealthState>,
    rx: watch::Receiver<HealthState>,
}

impl HealthBroadcaster {
    /// Create a new broadcaster with an initial health state.
    pub fn new(initial: HealthState) -> Self {
        let (tx, rx) = watch::channel(initial);
        Self { tx, rx }
    }

    /// Update the health state. All subscribers are notified.
    pub fn update(&self, state: HealthState) -> Result<(), watch::error::SendError<HealthState>> {
        self.tx.send(state)
    }

    /// Get a new subscriber receiver.
    pub fn subscribe(&self) -> watch::Receiver<HealthState> {
        self.rx.clone()
    }

    /// Get the current health state without subscribing.
    pub fn current(&self) -> HealthState {
        *self.rx.borrow()
    }
}

/// Backpressure-aware telemetry pipeline using `tokio::sync::mpsc`.
///
/// The producer (Kafka consumer) sends telemetry JSON into the channel.
/// The consumer (processing engine) processes at its own pace. If the
/// channel is full, the producer blocks — providing natural backpressure.
pub struct TelemetryPipeline {
    tx: mpsc::Sender<Vec<u8>>,
    rx: mpsc::Receiver<Vec<u8>>,
}

impl TelemetryPipeline {
    /// Create a new pipeline with the given buffer capacity.
    ///
    /// `capacity` controls backpressure: once the buffer is full,
    /// senders will await until space is available.
    pub fn new(capacity: usize) -> Self {
        let (tx, rx) = mpsc::channel(capacity);
        Self { tx, rx }
    }

    /// Get a sender handle. Can be cloned for multiple producers.
    pub fn sender(&self) -> mpsc::Sender<Vec<u8>> {
        self.tx.clone()
    }

    /// Take ownership of the receiver. Only one consumer allowed.
    pub fn into_receiver(self) -> mpsc::Receiver<Vec<u8>> {
        self.rx
    }
}

/// Concurrency limiter using `tokio::sync::Semaphore`.
///
/// Limits the number of concurrent database queries or API calls
/// to prevent overwhelming downstream services.
///
/// # Examples
/// ```ignore
/// let limiter = ConcurrencyLimiter::new(10); // Max 10 concurrent ops
///
/// let _permit = limiter.acquire().await.unwrap();
/// // ... do database query ...
/// // permit is dropped here, releasing the slot
/// ```
pub struct ConcurrencyLimiter {
    semaphore: Arc<Semaphore>,
    max_permits: usize,
}

impl ConcurrencyLimiter {
    /// Create a new limiter with the given maximum concurrency.
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_permits: max_concurrent,
        }
    }

    /// Acquire a permit. Blocks if all permits are taken.
    pub async fn acquire(
        &self,
    ) -> Result<tokio::sync::SemaphorePermit<'_>, tokio::sync::AcquireError> {
        self.semaphore.acquire().await
    }

    /// Try to acquire a permit without blocking.
    pub fn try_acquire(
        &self,
    ) -> Result<tokio::sync::SemaphorePermit<'_>, tokio::sync::TryAcquireError> {
        self.semaphore.try_acquire()
    }

    /// Returns the number of available permits.
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Returns the maximum number of permits.
    pub fn max_permits(&self) -> usize {
        self.max_permits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_broadcaster_basic() {
        let broadcaster = HealthBroadcaster::new(HealthState::Green);
        assert_eq!(broadcaster.current(), HealthState::Green);

        let rx = broadcaster.subscribe();
        broadcaster.update(HealthState::Yellow).unwrap();
        assert_eq!(*rx.borrow(), HealthState::Yellow);

        broadcaster.update(HealthState::Red).unwrap();
        assert_eq!(*rx.borrow(), HealthState::Red);
    }

    #[tokio::test]
    async fn health_broadcaster_multiple_subscribers() {
        let broadcaster = HealthBroadcaster::new(HealthState::Green);
        let rx1 = broadcaster.subscribe();
        let rx2 = broadcaster.subscribe();

        broadcaster.update(HealthState::Red).unwrap();
        assert_eq!(*rx1.borrow(), HealthState::Red);
        assert_eq!(*rx2.borrow(), HealthState::Red);
    }

    #[tokio::test]
    async fn telemetry_pipeline_send_receive() {
        let pipeline = TelemetryPipeline::new(10);
        let tx = pipeline.sender();
        let mut rx = pipeline.into_receiver();

        tx.send(b"test payload".to_vec()).await.unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(received, b"test payload");
    }

    #[tokio::test]
    async fn concurrency_limiter_basic() {
        let limiter = ConcurrencyLimiter::new(2);
        assert_eq!(limiter.available_permits(), 2);

        let _p1 = limiter.acquire().await.unwrap();
        assert_eq!(limiter.available_permits(), 1);

        let _p2 = limiter.acquire().await.unwrap();
        assert_eq!(limiter.available_permits(), 0);

        // Third acquire would block — test try_acquire instead
        assert!(limiter.try_acquire().is_err());

        drop(_p1);
        assert_eq!(limiter.available_permits(), 1);
    }
}
