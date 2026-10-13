use crate::{credential_store::CredentialStore, error::Result};
use serde_json::Value;
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};
pub struct NoCredentials;
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
pub fn server(responses: Vec<(u16, Value)>) -> (String, thread::JoinHandle<Vec<String>>) {
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
            // macOS can inherit nonblocking mode from the listener on accepted sockets.
            stream.set_nonblocking(false).unwrap();
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
