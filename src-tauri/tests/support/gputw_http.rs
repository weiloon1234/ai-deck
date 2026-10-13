use super::*;
use crate::http_fixture::{server, NoCredentials};
use serde_json::json;

#[tokio::test]
async fn gputw_check_only_reads_instances_and_discards_private_records() {
    let (base, worker) = server(vec![(
        200,
        json!({"success":true,"data":[{"id":"fixture","env":{"private":"fixture-private-value"}}]}),
    )]);
    let client = GputwClient::new(Arc::new(NoCredentials)).unwrap();
    client
        .check_at(&format!("{base}/api/instances"), "fixture-gputw-key")
        .await
        .unwrap();
    let requests = worker.join().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("GET /api/instances HTTP/1.1"));
    assert!(requests[0].contains("Bearer fixture-gputw-key"));
    // GPUtw's front door requires an identifiable User-Agent even with a valid key.
    assert!(requests[0].contains("user-agent: AI-Deck/"));
    assert!(requests[0].contains("accept: application/json"));
    assert!(!requests[0].contains("fixture-private-value"));
}

#[tokio::test]
async fn gputw_check_rejects_auth_redirects_and_invalid_envelopes_without_echoing_bodies() {
    let client = GputwClient::new(Arc::new(NoCredentials)).unwrap();
    for (status, body, code) in [
        (
            403,
            json!({"error":"fixture-private-value"}),
            "authentication",
        ),
        (
            302,
            json!({"error":"fixture-private-value"}),
            "provider_unavailable",
        ),
        (
            200,
            json!({"success":false,"data":[],"error":"fixture-private-value"}),
            "provider_protocol",
        ),
        (
            200,
            json!({"success":true,"data":["fixture-private-value"]}),
            "provider_protocol",
        ),
        (200, json!({"success":true,"data":{}}), "provider_protocol"),
    ] {
        let (base, worker) = server(vec![(status, body)]);
        let error = client
            .check_at(&base, "fixture-gputw-key")
            .await
            .unwrap_err();
        assert_eq!(error.code, code);
        assert!(!serde_json::to_string(&error)
            .unwrap()
            .contains("fixture-private-value"));
        assert_eq!(worker.join().unwrap().len(), 1);
    }
}
