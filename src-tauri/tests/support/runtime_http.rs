// Included as a module by runtime_client so the production endpoint remains fixed.
use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};
struct NoCredentials;
impl CredentialStore for NoCredentials {
    fn get(&self, _: &str) -> Result<Option<String>> {
        panic!("test uses explicit fake credential")
    }
    fn set(&self, _: &str, _: &str) -> Result<()> {
        unreachable!()
    }
    fn delete(&self, _: &str) -> Result<()> {
        unreachable!()
    }
}
fn server(responses: Vec<(u16, Value)>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener =
        TcpListener::bind("127.0.0.1:0").expect("local HTTP fixture needs loopback permission");
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let worker = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut requests = vec![];
        for (code, body) in responses {
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
                    Err(error) => panic!("fixture timed out: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = vec![];
            let mut buffer = [0; 1024];
            loop {
                let n = stream.read(&mut buffer).unwrap();
                bytes.extend_from_slice(&buffer[..n]);
                if bytes.windows(4).any(|p| p == b"\r\n\r\n") || n == 0 {
                    break;
                }
            }
            requests.push(String::from_utf8_lossy(&bytes).to_string());
            let body = body.to_string();
            let head = format!("HTTP/1.1 {code} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            stream.write_all(head.as_bytes()).unwrap();
            stream.write_all(body.as_bytes()).unwrap();
        }
        requests
    });
    (format!("http://{address}"), worker)
}
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
