use std::future::Future;
use std::pin::Pin;

pub trait TelemetryProcessor: Send + Sync {
    fn process<'a>(
        &'a self,
        sat_id: &'a str,
        record: &'a spaceops_models::TelemetryRecord,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
}
