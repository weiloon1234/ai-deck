//! Explicit opt-in smoke test. No GPU or cloud inference: both CLIs get fake
//! credentials, a disposable HOME/project, and a loopback rejection endpoint.
mod support;
use ai_deck::{
    cli_adapters::{self, claude_adapter, codex_adapter},
    types::*,
};
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use support::*;

#[cfg(unix)]
#[test]
#[ignore = "requires installed pinned Codex and loopback permission; no subscription or paid endpoint"]
fn installed_codex_accepts_analysis_schema_and_restricts_tools() {
    use ai_deck::codex_model_analyzer::{analysis_arguments, output_schema};
    use std::os::unix::process::CommandExt;
    let installation = cli_adapters::discover(CliKind::Codex, None);
    cli_adapters::assert_known_launcher(&CliKind::Codex, installation.version.as_deref().unwrap())
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("disposable-home");
    std::fs::create_dir_all(home.join(".codex")).unwrap();
    std::fs::write(
        root.path().join("schema.json"),
        serde_json::to_vec(&output_schema()).unwrap(),
    )
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let stopped = Arc::new(AtomicBool::new(false));
    let stop = stopped.clone();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let seen = requests.clone();
    let server = std::thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let mut bytes = vec![];
            let mut buffer = [0; 8192];
            while let Ok(n) = stream.read(&mut buffer) {
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(end) = bytes.windows(4).position(|p| p == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
                if bytes.len() > 2 * 1024 * 1024 {
                    break;
                }
            }
            seen.lock().unwrap().push(bytes);
            let body = r#"{"error":{"message":"Fixture rejects all inference"}}"#;
            let _ = write!(stream, "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        }
    });
    let mut args = analysis_arguments(root.path());
    args.pop();
    // Only the test replaces the subscription provider with a no-auth localhost fixture.
    args.extend(["--config".into(), "model_provider=\"analysis_fixture\"".into(), "--config".into(),
        format!("model_providers.analysis_fixture={{name=\"Fixture\",base_url=\"{base}\",wire_api=\"responses\",requires_openai_auth=false}}"),
        "--config".into(), "features.enable_request_compression=false".into(), "--config".into(), "cli_auth_credentials_store=\"file\"".into(),
        "Return a hardware suggestion.".into()]);
    let output_path = root.path().join("output.txt");
    let output = std::fs::File::create(&output_path).unwrap();
    let mut child = Command::new(installation.path.unwrap())
        .args(args)
        .env_clear()
        .env("HOME", &home)
        .env("CODEX_HOME", home.join(".codex"))
        .env("PATH", "/usr/bin:/bin")
        .current_dir(root.path())
        .stdin(Stdio::null())
        .stdout(output.try_clone().unwrap())
        .stderr(output)
        .process_group(0)
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(25);
    let mut ended = false;
    while Instant::now() < deadline {
        if child.try_wait().unwrap().is_some() {
            ended = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    if !ended {
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
    }
    let _ = child.wait();
    stopped.store(true, Ordering::SeqCst);
    server.join().unwrap();
    let requests = requests.lock().unwrap();
    let output = std::fs::read_to_string(output_path).unwrap();
    assert!(
        !requests.is_empty(),
        "Codex failed before reaching localhost: {}",
        output.chars().take(2200).collect::<String>()
    );
    let mut checked = false;
    for bytes in requests.iter() {
        let Some(end) = bytes.windows(4).position(|p| p == b"\r\n\r\n") else {
            continue;
        };
        if let Ok(body) = serde_json::from_slice::<serde_json::Value>(&bytes[end + 4..]) {
            if body["text"]["format"]["type"] != "json_schema" {
                continue;
            }
            assert!(body["tools"]
                .as_array()
                .is_none_or(|tools| tools.is_empty()));
            // 0.160 sends tool declarations as input items, not top-level tools.
            for namespace in body["input"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|i| i["type"] == "additional_tools")
                .flat_map(|i| i["tools"].as_array().into_iter().flatten())
            {
                assert_eq!(namespace["name"], "functions", "Unexpected tool namespace");
                for tool in namespace["tools"].as_array().into_iter().flatten() {
                    assert!(
                        [
                            "exec",
                            "wait",
                            "request_user_input",
                            "request_user_input_async"
                        ]
                        .contains(&tool["name"].as_str().unwrap()),
                        "Unexpected direct tool"
                    );
                }
            }
            checked = true;
        }
    }
    assert!(checked, "No structured analysis request was captured");
    // The CLI retains a wrapper declaration; its executor must fail closed.
    assert!(output.contains("code-mode host is disabled"));
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "requires installed pinned CLIs and loopback permission; never calls a paid endpoint"]
async fn installed_clis_route_invalid_credentials_only_to_the_fixture_gateway() {
    use std::os::unix::process::CommandExt;
    let f = Fixture::new();
    let id = f.ready().await;
    let d = f.core.deployment(&id).unwrap();
    for cli in [CliKind::Codex, CliKind::Claude] {
        let installation = cli_adapters::discover(cli.clone(), None);
        cli_adapters::assert_known_launcher(
            &cli,
            installation
                .version
                .as_deref()
                .expect("CLI must be installed"),
        )
        .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = stopped.clone();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = requests.clone();
        let server = std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let mut bytes = vec![];
                let mut buffer = [0; 8192];
                while let Ok(n) = stream.read(&mut buffer) {
                    if n == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&buffer[..n]);
                    if bytes.windows(4).any(|p| p == b"\r\n\r\n") || bytes.len() > 256 * 1024 {
                        break;
                    }
                }
                let text = String::from_utf8_lossy(&bytes).to_string();
                seen.lock().unwrap().push(text);
                let body = "{\"type\":\"error\",\"error\":{\"type\":\"authentication_error\",\"message\":\"Fixture intentionally rejects credentials\"}}";
                let response = format!("HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                let _ = stream.write_all(response.as_bytes());
            }
        });
        let root = f.directory.path().join(format!("installed-{cli:?}"));
        std::fs::create_dir(&root).unwrap();
        let project = root.join("project with spaces 日本語");
        std::fs::create_dir(&project).unwrap();
        let home = root.join("normal-home");
        std::fs::create_dir_all(home.join(".codex")).unwrap();
        let normal_auth = b"{\"OPENAI_API_KEY\":\"normal-provider-fixture-must-never-be-used\"}";
        std::fs::write(home.join(".codex/auth.json"), normal_auth).unwrap();
        let session = Session {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Fixture".into(),
            project_id: "fixture".into(),
            project_path: project.to_string_lossy().into(),
            cli: cli.clone(),
            cli_version: installation.version.unwrap(),
            deployment_id: id.clone(),
            profile_id: d.profile.model.id.clone(),
            profile_version: d.profile.model.version,
            model: d.profile.model.served_model.clone(),
            status: SessionStatus::Starting,
            created_at: now(),
            ended_at: None,
            conversation_id: None,
            config_directory: root.join("app-state").to_string_lossy().into(),
            unverified_combination: true,
            exit_code: None,
        };
        let token = "fixture-runpod-credential";
        let mut spec = match cli {
            CliKind::Codex => codex_adapter::prepare(
                installation.path.unwrap().into(),
                &session,
                &d,
                token,
                false,
            ),
            CliKind::Claude => claude_adapter::prepare(
                installation.path.unwrap().into(),
                &session,
                &d,
                token,
                false,
            ),
        }
        .unwrap();
        spec.environment
            .insert("HOME".into(), home.to_string_lossy().into());
        let remote =
            ai_deck::runtime_client::endpoint(&d, d.profile.runtime.inference_port).unwrap();
        for value in spec.environment.values_mut() {
            *value = value.replace(&remote, &base);
        }
        for value in &mut spec.arguments {
            *value = value.replace(&remote, &base);
        }
        for entry in std::fs::read_dir(&session.config_directory).unwrap() {
            let path = entry.unwrap().path();
            let value = std::fs::read_to_string(&path).unwrap();
            std::fs::write(path, value.replace(&remote, &base)).unwrap();
        }
        assert!(!spec.arguments.join(" ").contains("proxy.runpod.net"));
        assert!(!spec
            .environment
            .values()
            .any(|v| v.contains("proxy.runpod.net")));
        match cli {
            CliKind::Codex => spec
                .arguments
                .extend(["exec", "--skip-git-repo-check", "Reply OK."].map(String::from)),
            CliKind::Claude => spec
                .arguments
                .extend(["--print", "--max-turns", "1", "Reply OK."].map(String::from)),
        }
        let output_path = root.join("process-output.txt");
        let output = std::fs::File::create(&output_path).unwrap();
        let mut child = Command::new(spec.executable)
            .args(spec.arguments)
            .env_clear()
            .envs(spec.environment)
            .current_dir(spec.directory)
            .stdin(Stdio::null())
            .stdout(output.try_clone().unwrap())
            .stderr(output)
            .process_group(0)
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(25);
        while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
        let _ = child.wait();
        stopped.store(true, Ordering::SeqCst);
        server.join().unwrap();
        let requests = requests.lock().unwrap();
        let output = std::fs::read_to_string(output_path).unwrap();
        assert!(
            !requests.is_empty(),
            "{cli:?} never reached the local gateway: {}",
            output.chars().take(1800).collect::<String>()
        );
        assert!(
            requests.iter().any(|r| r.contains(token)),
            "{cli:?} did not send its scoped credential"
        );
        assert!(requests
            .iter()
            .all(|r| !r.contains("normal-provider-fixture-must-never-be-used")));
        assert_eq!(
            std::fs::read(home.join(".codex/auth.json")).unwrap(),
            normal_auth
        );
        println!("{cli:?}: installed adapter reached localhost with its scoped fake credential; normal fixture auth unchanged.");
    }
}
