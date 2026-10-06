use ai_deck::{
    cli_adapters::{controlled_environment, LaunchSpec},
    terminal_process::{StreamRedactor, TerminalManager, Utf8Decoder},
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
#[test]
fn streaming_redaction_handles_every_split_and_unicode_decoder_preserves_codepoints() {
    let secret = "rpd_a1b2c3d4";
    for split in 0..=secret.len() {
        let mut redactor = StreamRedactor::new(vec![secret.into()]);
        let a = redactor.push(&format!("before {}", &secret[..split]), false);
        let b = redactor.push(&format!("{} after", &secret[split..]), true);
        assert_eq!(format!("{a}{b}"), "before [REDACTED] after");
    }
    let input = "ASCII 日本語 👩🏽‍💻\n";
    let mut decoder = Utf8Decoder::default();
    let mut output = String::new();
    for byte in input.as_bytes() {
        output.push_str(&decoder.push(&[*byte], false));
    }
    output.push_str(&decoder.push(&[], true));
    assert_eq!(output, input);
    assert_eq!(decoder.push(&[0xff, b'x'], true), "�x");
}
#[cfg(unix)]
#[tokio::test]
async fn real_pty_preserves_input_unicode_resize_approval_and_process_cleanup() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("a project 日本語");
    std::fs::create_dir(&project).unwrap();
    let manager = TerminalManager::default();
    let exited = Arc::new(AtomicBool::new(false));
    let done = exited.clone();
    let spec = LaunchSpec { executable: PathBuf::from("/bin/sh"), arguments: vec!["-c".into(), "printf 'APPROVE? '; read answer; printf 'ANSWER:%s 日本語\\n' \"$answer\"; pwd; stty size; sleep 30".into()], environment: controlled_environment(), directory: project.clone(), redacted_values: vec![] };
    manager
        .spawn(
            "one",
            spec,
            Arc::new(|_, _| {}),
            Arc::new(move |_| {
                done.store(true, Ordering::SeqCst);
            }),
        )
        .unwrap();
    manager.resize("one", 41, 123).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let text: String = manager
            .replay("one")
            .unwrap()
            .chunks
            .into_iter()
            .map(|c| c.text)
            .collect();
        if text.contains("APPROVE?") {
            break;
        }
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    manager.input("one", "yes\n").unwrap();
    loop {
        let text: String = manager
            .replay("one")
            .unwrap()
            .chunks
            .into_iter()
            .map(|c| c.text)
            .collect();
        if text.contains("ANSWER:yes 日本語")
            && text.contains("41 123")
            && text.contains("a project 日本語")
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "PTY did not emit expected output: {text}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    manager.input("one", "\u{3}").unwrap();
    manager.shutdown().await.unwrap();
    while !exited.load(Ordering::SeqCst) {
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(!manager.is_running("one"));
    assert!(manager.input("one", "x").is_err());
    let previous = manager
        .replay("one")
        .unwrap()
        .chunks
        .last()
        .unwrap()
        .sequence;
    manager
        .spawn(
            "one",
            LaunchSpec {
                executable: PathBuf::from("/bin/echo"),
                arguments: vec!["resumed".into()],
                environment: controlled_environment(),
                directory: project,
                redacted_values: vec![],
            },
            Arc::new(|_, _| {}),
            Arc::new(|_| {}),
        )
        .unwrap();
    loop {
        let chunks = manager.replay("one").unwrap().chunks;
        if chunks.iter().any(|c| c.text.contains("resumed")) {
            assert!(chunks[0].sequence > previous);
            break;
        }
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn detached_tools_stop_on_close_or_parent_exit_without_affecting_other_sessions() {
    struct FixtureCleanup(tempfile::TempDir);
    impl Drop for FixtureCleanup {
        fn drop(&mut self) {
            let _ = std::fs::write(self.0.path().join("stop"), "stop");
            // A failed assertion still asks only these bounded fixtures to stop.
            std::thread::sleep(Duration::from_millis(150));
        }
    }
    let fixture = FixtureCleanup(tempfile::tempdir().unwrap());
    let script = fixture.0.path().join("detached-tool.py");
    std::fs::write(
        &script,
        r#"import pathlib, signal, subprocess, sys, time
root = pathlib.Path(sys.argv[1])
name, mode = sys.argv[2:4]
if mode == 'tool':
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    for _ in range(600):
        if (root / 'stop').exists(): break
        (root / (name + '.heartbeat')).write_text(str(time.monotonic()))
        time.sleep(0.05)
else:
    subprocess.Popen([sys.executable, __file__, str(root), name, 'tool'], start_new_session=True)
    while not (root / (name + '.heartbeat')).exists(): time.sleep(0.01)
    if mode != 'exit':
        for _ in range(600):
            if (root / 'stop').exists(): break
            time.sleep(0.05)
"#,
    )
    .unwrap();
    let manager = TerminalManager::default();
    let spawn = |id: &str, mode: &str| {
        manager
            .spawn(
                id,
                LaunchSpec {
                    executable: PathBuf::from("/usr/bin/python3"),
                    arguments: vec![
                        script.to_string_lossy().into(),
                        fixture.0.path().to_string_lossy().into(),
                        id.into(),
                        mode.into(),
                    ],
                    environment: controlled_environment(),
                    directory: fixture.0.path().into(),
                    redacted_values: vec![],
                },
                Arc::new(|_, _| {}),
                Arc::new(|_| {}),
            )
            .unwrap();
    };
    let heartbeat = |id: &str| {
        std::fs::read_to_string(fixture.0.path().join(format!("{id}.heartbeat"))).unwrap()
    };
    spawn("other", "keep");
    for mode in ["close", "exit"] {
        spawn(mode, mode);
        let deadline = Instant::now() + Duration::from_secs(10);
        while !fixture.0.path().join(format!("{mode}.heartbeat")).exists() {
            assert!(Instant::now() < deadline);
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        if mode == "close" {
            manager.close(mode).unwrap();
        }
        while manager.is_running(mode) {
            assert!(Instant::now() < deadline, "cleanup did not finish");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let stopped = heartbeat(mode);
        let other = heartbeat("other");
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert_eq!(
            heartbeat(mode),
            stopped,
            "detached tool still writes after session ended"
        );
        assert_ne!(
            heartbeat("other"),
            other,
            "unrelated session was interrupted"
        );
    }
    manager.shutdown().await.unwrap();
    let stopped = heartbeat("other");
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(heartbeat("other"), stopped);
}
