// Included as a module by runtime_client so the production endpoint remains fixed.
use super::*;
use crate::http_fixture::{server, NoCredentials};
#[tokio::test]
async fn ready_requires_rejected_bad_auth_selected_model_and_actual_inference() {
    let (base, server) = server(vec![
        (401, json!({"error":"rejected"})),
        (200, json!({"data":[{"id":"selected"}]})),
        (
            200,
            json!({"model":"selected","status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"OK"}]}]}),
        ),
    ]);
    let client = RuntimeClient::new(Arc::new(NoCredentials)).unwrap();
    client
        .verify_at(&base, "selected", "fixture-token")
        .await
        .unwrap();
    let requests = server.join().unwrap();
    assert!(requests[0].contains("ai-deck-invalid-probe"));
    assert!(!requests[0].contains("fixture-token"));
    assert!(requests[1].contains("fixture-token"));
    assert!(requests[2].starts_with("POST /v1/responses"));
}
#[tokio::test]
async fn unprotected_endpoint_wrong_model_and_failed_inference_are_rejected() {
    let client = RuntimeClient::new(Arc::new(NoCredentials)).unwrap();
    for (responses, code) in [
        (vec![(200, json!({}))], "endpoint_not_protected"),
        (
            vec![(401, json!({})), (200, json!({"data":[{"id":"wrong"}]}))],
            "wrong_model",
        ),
        (
            vec![
                (401, json!({})),
                (200, json!({"data":[{"id":"selected"}]})),
                (
                    200,
                    json!({"model":"selected","status":"failed","output":[]}),
                ),
            ],
            "inference_verification",
        ),
        (
            vec![
                (401, json!({})),
                (403, json!({"error":"sensitive upstream body"})),
            ],
            "authentication",
        ),
    ] {
        let (base, server) = server(responses);
        let error = client
            .verify_at(&base, "selected", "fixture-token")
            .await
            .unwrap_err();
        assert_eq!(error.code, code);
        assert!(!error.message.contains("sensitive"));
        server.join().unwrap();
    }
}

#[tokio::test]
async fn transient_invalid_auth_probe_is_inconclusive_and_can_recover() {
    let client = RuntimeClient::new(Arc::new(NoCredentials)).unwrap();
    for status in [302, 429, 502, 503, 504] {
        let (base, worker) = server(vec![(status, json!({"error":"temporary"}))]);
        let error = client
            .verify_at(&base, "selected", "fixture-token")
            .await
            .unwrap_err();
        assert_eq!(
            error.code,
            if status == 429 {
                "rate_limit"
            } else {
                "provider_unavailable"
            }
        );
        assert_eq!(worker.join().unwrap().len(), 1);
    }
}
