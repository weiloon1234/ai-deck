mod support;
use ai_deck::{
    cli_adapters::{self, claude_adapter, codex_adapter},
    types::*,
};
use std::{collections::HashSet, path::PathBuf};
use support::*;

fn session(f: &Fixture, d: &Deployment, cli: CliKind) -> Session {
    let project = f.directory.path().join("project with spaces 日本語");
    std::fs::create_dir_all(&project).unwrap();
    Session {
        id: "fixture-session".into(),
        name: "Test".into(),
        project_id: "project".into(),
        project_path: project.to_string_lossy().into(),
        cli,
        cli_version: "fixture".into(),
        deployment_id: d.id.clone(),
        profile_id: d.profile.model.id.clone(),
        profile_version: 1,
        model: d.profile.model.served_model.clone(),
        status: SessionStatus::Starting,
        created_at: now(),
        ended_at: None,
        conversation_id: None,
        config_directory: f
            .directory
            .path()
            .join("isolated config")
            .to_string_lossy()
            .into(),
        unverified_combination: true,
        exit_code: None,
    }
}
#[tokio::test]
async fn adapters_keep_secrets_out_of_files_and_arguments_and_preserve_local_paths() {
    let f = Fixture::new();
    let id = f.ready().await;
    let d = f.core.deployment(&id).unwrap();
    let token = "fixture-inference-secret-never-a-real-key";
    for cli in [CliKind::Codex, CliKind::Claude] {
        let s = session(&f, &d, cli.clone());
        let spec = match cli {
            CliKind::Codex => {
                codex_adapter::prepare(PathBuf::from("/fixture/codex"), &s, &d, token, true)
            }
            CliKind::Claude => {
                claude_adapter::prepare(PathBuf::from("/fixture/claude"), &s, &d, token, true)
            }
        }
        .unwrap();
        assert_eq!(spec.directory.to_string_lossy(), s.project_path);
        assert!(!spec.arguments.join(" ").contains(token));
        for entry in std::fs::read_dir(&s.config_directory).unwrap() {
            assert!(!std::fs::read_to_string(entry.unwrap().path())
                .unwrap()
                .contains(token));
        }
        assert!(!spec.environment.contains_key("OPENAI_API_KEY"));
        assert!(!spec.environment.contains_key("CLAUDE_CODE_OAUTH_TOKEN"));
        assert!(!spec.environment.contains_key("RUNPOD_API_KEY"));
        assert!(!spec.environment.contains_key("HF_TOKEN"));
        match cli {
            CliKind::Codex => {
                assert!(spec.arguments.contains(&"--no-daemon".into()));
                assert!(spec.arguments.contains(&"on-request".into()));
                assert!(spec.arguments.contains(&s.project_path));
                assert!(spec
                    .arguments
                    .ends_with(&["resume".into(), "--last".into()]));
                assert_eq!(spec.environment["CODEX_HOME"], s.config_directory);
                assert_eq!(spec.environment["RUNPOD_DECK_INFERENCE_TOKEN"], token);
            }
            CliKind::Claude => {
                assert!(spec.arguments.contains(&"--bare".into()));
                assert!(spec.arguments.contains(&"--strict-mcp-config".into()));
                assert!(spec
                    .arguments
                    .windows(2)
                    .any(|p| p == ["--setting-sources", ""]));
                assert!(spec
                    .arguments
                    .windows(2)
                    .any(|p| p == ["--permission-mode", "manual"]));
                for key in [
                    "ANTHROPIC_MODEL",
                    "ANTHROPIC_DEFAULT_OPUS_MODEL",
                    "ANTHROPIC_DEFAULT_SONNET_MODEL",
                    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
                    "ANTHROPIC_SMALL_FAST_MODEL",
                    "CLAUDE_CODE_SUBAGENT_MODEL",
                ] {
                    assert_eq!(spec.environment[key], d.profile.model.served_model);
                }
                assert_eq!(spec.environment["CLAUDE_CONFIG_DIR"], s.config_directory);
            }
        }
    }
}
#[test]
fn inherited_environment_is_an_allowlist_and_new_cli_versions_are_not_trusted() {
    let allowed: HashSet<_> = [
        "PATH",
        "HOME",
        "USER",
        "LOGNAME",
        "TMPDIR",
        "LANG",
        "LC_ALL",
        "LC_CTYPE",
        "SHELL",
        "TERM",
        "COLORTERM",
    ]
    .into_iter()
    .collect();
    assert!(cli_adapters::controlled_environment()
        .keys()
        .all(|key| allowed.contains(key.as_str())));
    assert!(cli_adapters::assert_known_launcher(&CliKind::Codex, "codex-cli 0.160.0").is_ok());
    assert!(cli_adapters::assert_known_launcher(&CliKind::Codex, "codex-cli 0.160.1").is_err());
}
#[tokio::test]
async fn no_route_or_unavailable_original_binding_cannot_launch_or_resume() {
    let f = Fixture::new();
    assert_eq!(
        f.core
            .launch(LaunchRequest {
                project_id: "x".into(),
                cli: CliKind::Codex,
                name: "x".into(),
                allow_unverified: true
            })
            .await
            .unwrap_err()
            .code,
        "no_enabled_endpoint"
    );
    let id = f.ready().await;
    let d = f.core.deployment(&id).unwrap();
    let s = session(&f, &d, CliKind::Codex);
    let sid = s.id.clone();
    f.core
        .store
        .update(|state| {
            state.sessions.push(s);
            Ok(())
        })
        .unwrap();
    f.core.finish(&id).await.unwrap();
    assert!(f.core.resume(&sid, true).await.is_err());
    assert!(!f.core.terminals.is_running(&sid));
}

#[cfg(unix)]
#[tokio::test]
async fn two_fixture_clis_keep_independent_projects_history_capacity_and_resume_binding() {
    use std::{
        os::unix::fs::PermissionsExt,
        time::{Duration, Instant},
    };
    let f = Fixture::new();
    let id = f.ready().await;
    f.core.enable(&id).await.unwrap();
    let executable = f.directory.path().join("fixture cli");
    std::fs::write(&executable, "#!/bin/sh\nif [ \"$1\" = '--version' ]; then printf 'codex-cli 0.160.0\\n'; exit 0; fi\nprintf 'CWD:%s\\nSTATE:%s\\nTOKEN:%s\\n' \"$PWD\" \"$CODEX_HOME\" \"$RUNPOD_DECK_INFERENCE_TOKEN\"\nwhile read -r value; do printf 'APPROVED:%s\\n' \"$value\"; done\n").unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let a = f.directory.path().join("project A 日本語");
    let b = f.directory.path().join("project B");
    std::fs::create_dir(&a).unwrap();
    std::fs::create_dir(&b).unwrap();
    f.core
        .store
        .update(|s| {
            s.settings.codex_path = Some(executable.to_string_lossy().into());
            s.settings.policy.auto_terminate_delay_seconds = Some(30);
            s.projects = vec![
                Project {
                    id: "a".into(),
                    name: "A".into(),
                    path: a.to_string_lossy().into(),
                },
                Project {
                    id: "b".into(),
                    name: "B".into(),
                    path: b.to_string_lossy().into(),
                },
            ];
            Ok(())
        })
        .unwrap();
    let request = |project: &str| LaunchRequest {
        project_id: project.into(),
        cli: CliKind::Codex,
        name: project.into(),
        allow_unverified: true,
    };
    let first = f.core.launch(request("a")).await.unwrap();
    let second = f.core.launch(request("b")).await.unwrap();
    assert_eq!(
        f.core.launch(request("a")).await.unwrap_err().code,
        "session_capacity"
    );
    let deadline = Instant::now() + Duration::from_secs(8);
    for (sid, path) in [(&first, &a), (&second, &b)] {
        loop {
            let text: String = f
                .core
                .terminals
                .replay(sid)
                .unwrap()
                .chunks
                .into_iter()
                .map(|c| c.text)
                .collect();
            if text.contains("TOKEN:[REDACTED]") {
                assert!(text.contains(path.to_str().unwrap()));
                assert!(text.contains(sid));
                break;
            }
            assert!(Instant::now() < deadline);
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
    let sessions = f.core.store.read().unwrap().sessions;
    assert_ne!(sessions[0].config_directory, sessions[1].config_directory);
    f.core.close_session(&first).unwrap();
    f.core.close_session(&second).unwrap();
    loop {
        if f.core
            .store
            .read()
            .unwrap()
            .sessions
            .iter()
            .all(|s| s.status == SessionStatus::Ended)
        {
            break;
        }
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        f.core.deployment(&id).unwrap().stage,
        DeploymentStage::Ready
    );
    assert!(f.core.deployment(&id).unwrap().cleanup_due_at.is_some());
    // Changing the current selection must never redirect an old conversation.
    f.core
        .store
        .update(|s| {
            s.enabled_deployment_id = Some("different-selection".into());
            Ok(())
        })
        .unwrap();
    f.core.resume(&first, true).await.unwrap();
    assert_eq!(f.core.store.read().unwrap().sessions[0].deployment_id, id);
    assert!(f.core.deployment(&id).unwrap().cleanup_due_at.is_none());
    f.core.finish(&id).await.unwrap();
    assert!(!f.core.terminals.is_running(&first));
    assert!(!f.core.terminals.is_running(&second));
}
