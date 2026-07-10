use std::future::Future;
use std::pin::Pin;

pub trait TelemetryProcessor: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    type Output: Send + 'static;

    fn process<'a>(
        &'a self,
        sat_id: &'a str,
        record: &'a orbitlogs_models::TelemetryRecord,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send + 'a>>;
}

