use axum::body::{Body, to_bytes};
use axum::http::Request;
use prometheus_client::encoding::prometheus_protobuf::prometheus_data_model::MetricFamily;
use prost::Message;
use tower::ServiceExt;

use crate::metrics::Metrics;

pub(crate) async fn scrape(metrics: &Metrics) -> Vec<MetricFamily> {
    let response = crate::metrics::router(metrics.clone())
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .expect("a request"),
        )
        .await
        .expect("the metrics router is infallible");
    let mut bytes = to_bytes(response.into_body(), 16 * 1024 * 1024)
        .await
        .expect("a readable body");
    let mut families = Vec::new();
    while !bytes.is_empty() {
        families.push(
            MetricFamily::decode_length_delimited(&mut bytes).expect("a delimited metric family"),
        );
    }
    families
}

pub(crate) fn histogram_count(
    families: &[MetricFamily],
    name: &str,
    labels: &[(&str, &str)],
) -> u64 {
    families
        .iter()
        .filter(|family| family.name == name)
        .flat_map(|family| &family.metric)
        .filter(|metric| {
            labels.iter().all(|(key, value)| {
                metric
                    .label
                    .iter()
                    .any(|pair| pair.name == *key && pair.value == *value)
            })
        })
        .filter_map(|metric| metric.histogram.as_ref())
        .map(|histogram| histogram.sample_count)
        .sum()
}
