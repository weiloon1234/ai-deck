mod support;
use ai_deck::{model_catalog::*, state_store::StateStore, types::*};
use serde_json::Value;

fn catalog() -> Value {
    serde_json::from_str(include_str!("../../model-catalog/catalog.json")).unwrap()
}
#[test]
fn only_exact_evidence_survives_configuration_or_cli_changes() {
    let mut value = catalog();
    let original = builtin_catalog().unwrap().remove(0);
    assert!(!original.is_validated(
        &CliKind::Codex,
        "codex-cli 0.160.0",
        &original.model.gpu_types[0]
    ));
    value["profiles"][0]["evidence"] = serde_json::json!([{"cli":"codex","cliVersion":"codex-cli 0.160.0","gpuType":original.model.gpu_types[0],"fingerprint":original.fingerprint,
        "testedAt":"fixture-only","checks":REQUIRED_CODING_CHECKS,"report":"fixture-only; not distributed"}]);
    let attested = parse_catalog(&value.to_string()).unwrap().remove(0);
    assert!(attested.is_validated(
        &CliKind::Codex,
        "codex-cli 0.160.0",
        &original.model.gpu_types[0]
    ));
    assert!(!attested.is_validated(
        &CliKind::Codex,
        "codex-cli 0.161.0",
        &original.model.gpu_types[0]
    ));
    assert!(!attested.is_validated(
        &CliKind::Claude,
        "2.1.289 (Claude Code)",
        &original.model.gpu_types[0]
    ));
    value["profiles"][0]["contextTokens"] = 8192.into();
    assert!(!parse_catalog(&value.to_string()).unwrap()[0].is_validated(
        &CliKind::Codex,
        "codex-cli 0.160.0",
        &original.model.gpu_types[0]
    ));
}
#[test]
fn imports_reject_mutable_revisions_unpinned_images_and_unsafe_caches() {
    for (pointer, bad) in [
        ("/profiles/0/revision", "main"),
        ("/runtimes/0/image", "vllm/vllm-openai:latest"),
        (
            "/profiles/0/modelCachePath",
            "/workspace/cache/../credentials",
        ),
        ("/profiles/0/repository", "x/y;curl"),
    ] {
        let mut value = catalog();
        *value.pointer_mut(pointer).unwrap() = bad.into();
        assert!(
            parse_catalog(&value.to_string()).is_err(),
            "accepted {pointer}"
        );
    }
    let mut value = catalog();
    value["profiles"][0]["rawToken"] = "should-not-be-here".into();
    assert!(parse_catalog(&value.to_string()).is_err());
}
#[test]
fn corrupt_state_is_preserved_and_second_writer_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");
    std::fs::write(&path, b"not-json").unwrap();
    assert!(StateStore::open(dir.path().into()).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"not-json");
    std::fs::remove_file(path).unwrap();
    let store = StateStore::open(dir.path().into()).unwrap();
    assert!(StateStore::open(dir.path().into()).is_err());
    assert!(store
        .update(|s| {
            s.settings.policy.max_hourly_usd = 1.0;
            Err::<(), _>(ai_deck::error::AppError::invalid("rollback"))
        })
        .is_err());
    assert_eq!(
        store.read().unwrap().settings.policy.max_hourly_usd,
        HOURLY_CEILING_USD
    );
    drop(store);
    assert!(StateStore::open(dir.path().into()).is_ok());
}
#[test]
fn failed_persistence_does_not_publish_an_uncommitted_state() {
    let dir = tempfile::tempdir().unwrap();
    let store = StateStore::open(dir.path().into()).unwrap();
    let state_path = dir.path().join("state.json");
    std::fs::remove_file(&state_path).unwrap();
    std::fs::create_dir(&state_path).unwrap();
    assert!(store
        .update(|s| {
            s.enabled_deployment_id = Some("not-persisted".into());
            Ok(())
        })
        .is_err());
    assert!(store.read().unwrap().enabled_deployment_id.is_none());
}

#[tokio::test]
async fn restart_arms_idle_cleanup_and_never_extends_an_existing_countdown() {
    for existing in [None, Some(now() - 1)] {
        let f = support::Fixture::new();
        let id = f.ready().await;
        let d = f.core.deployment(&id).unwrap();
        f.core
            .store
            .update(|s| {
                s.settings.policy.auto_terminate_delay_seconds = Some(30);
                s.deployments[0].cleanup_due_at = existing;
                s.sessions.push(Session {
                    id: "fixture-session".into(),
                    name: "Fixture".into(),
                    project_id: "fixture".into(),
                    project_path: f.directory.path().to_string_lossy().into(),
                    cli: CliKind::Codex,
                    cli_version: "fixture".into(),
                    deployment_id: id.clone(),
                    profile_id: d.profile.model.id.clone(),
                    profile_version: d.profile.model.version,
                    model: d.profile.model.served_model.clone(),
                    status: SessionStatus::Running,
                    created_at: now(),
                    ended_at: None,
                    conversation_id: None,
                    config_directory: f.directory.path().to_string_lossy().into(),
                    unverified_combination: true,
                    exit_code: None,
                });
                Ok(())
            })
            .unwrap();
        let path = f.core.store.directory().to_path_buf();
        drop(f.core);
        drop(f.provider);
        let before = now();
        let reopened = StateStore::open(path.clone()).unwrap();
        let state = reopened.read().unwrap();
        assert_eq!(state.sessions[0].status, SessionStatus::Ended);
        let due = state.deployments[0].cleanup_due_at.unwrap();
        if let Some(existing) = existing {
            assert_eq!(due, existing);
        } else {
            assert!((before + 30..=now() + 30).contains(&due));
        }
        assert_eq!(state.deployments[0].startup_completed, Some(true));
        drop(reopened);
        assert_eq!(
            StateStore::open(path).unwrap().read().unwrap().deployments[0].cleanup_due_at,
            Some(due)
        );
    }
}

#[tokio::test]
async fn legacy_snapshots_load_without_rewriting_their_profile_and_preserve_known_readiness() {
    let f = support::Fixture::new();
    f.ready().await;
    let path = f.core.store.directory().to_path_buf();
    let mut old = serde_json::to_value(f.core.store.read().unwrap()).unwrap();
    let d = old["deployments"][0].as_object_mut().unwrap();
    d.remove("startupCompleted");
    d["profile"]["model"]
        .as_object_mut()
        .unwrap()
        .remove("minCacheGb");
    let fingerprint = d["profile"]["fingerprint"].clone();
    drop(f.core);
    drop(f.provider);
    std::fs::write(path.join("state.json"), serde_json::to_vec(&old).unwrap()).unwrap();
    let state = StateStore::open(path).unwrap().read().unwrap();
    assert_eq!(state.deployments[0].startup_completed, Some(true));
    assert_eq!(
        serde_json::json!(state.deployments[0].profile.fingerprint),
        fingerprint
    );
    assert!(state.deployments[0].verified_at.is_none());
    let mut catalog = catalog();
    catalog["profiles"][0]
        .as_object_mut()
        .unwrap()
        .remove("minCacheGb");
    assert!(parse_catalog(&catalog.to_string()).is_err());
}
