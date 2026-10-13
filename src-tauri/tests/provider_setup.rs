mod support;
use ai_deck::{
    cloud_provider::ProviderId,
    credential_store::{CredentialStore, GPUTW_KEY, RUNPOD_KEY},
    runtime_client::endpoint,
    state_store::StateStore,
};
use std::sync::atomic::Ordering;

#[tokio::test]
async fn provider_keys_are_separate_private_and_guarded_by_their_own_deployments() {
    let f = support::Fixture::new();
    f.core
        .save_provider_key(ProviderId::Runpod, "fixture-runpod-secret")
        .await
        .unwrap();
    let id = f.ready().await;
    f.core
        .save_provider_key(ProviderId::Gputw, "fixture-gputw-secret")
        .await
        .unwrap();
    assert_eq!(
        f.credentials.get(RUNPOD_KEY).unwrap().as_deref(),
        Some("fixture-runpod-secret")
    );
    assert_eq!(
        f.credentials.get(GPUTW_KEY).unwrap().as_deref(),
        Some("fixture-gputw-secret")
    );
    assert!(f
        .core
        .remove_provider_key(ProviderId::Runpod)
        .await
        .is_err());
    assert!(f
        .core
        .save_provider_key(ProviderId::Runpod, "fixture-other-account")
        .await
        .is_err());
    let snapshot = f.core.snapshot().unwrap();
    assert!(snapshot
        .providers
        .iter()
        .all(|p| p.key_configured == Some(true)));
    assert!(!serde_json::to_string(&snapshot)
        .unwrap()
        .contains("fixture-gputw-secret"));
    assert!(
        !std::fs::read_to_string(f.core.store.directory().join("state.json"))
            .unwrap()
            .contains("fixture-runpod-secret")
    );
    f.core.remove_provider_key(ProviderId::Gputw).await.unwrap();
    assert!(f.credentials.get(GPUTW_KEY).unwrap().is_none());
    assert!(f.credentials.get(RUNPOD_KEY).unwrap().is_some());
    f.core.finish(&id).await.unwrap();
    f.core
        .remove_provider_key(ProviderId::Runpod)
        .await
        .unwrap();
}

#[tokio::test]
async fn selecting_gputw_preserves_runpod_enable_recovery_cleanup_and_snapshot() {
    let f = support::Fixture::new();
    let id = f.ready().await;
    let original = f.core.deployment(&id).unwrap();
    f.core.select_provider(ProviderId::Gputw).await.unwrap();
    f.core.enable(&id).await.unwrap();
    f.core.reconcile_all().await.unwrap();
    let deployment = f.core.deployment(&id).unwrap();
    assert_eq!(deployment.provider, ProviderId::Runpod);
    assert_eq!(deployment.profile.fingerprint, original.profile.fingerprint);
    assert_eq!(
        endpoint(&deployment, 8000).unwrap(),
        "https://pod1-8000.proxy.runpod.net"
    );
    assert_eq!(
        f.core
            .store
            .read()
            .unwrap()
            .enabled_deployment_id
            .as_deref(),
        Some(id.as_str())
    );
    f.core.finish(&id).await.unwrap();
    assert_eq!(f.provider.deletes.load(Ordering::SeqCst), 1);
    let path = f.core.store.directory().to_path_buf();
    drop(f.core);
    drop(f.provider);
    let state = StateStore::open(path).unwrap().read().unwrap();
    assert_eq!(state.settings.provider, ProviderId::Gputw);
    assert_eq!(state.deployments[0].provider, ProviderId::Runpod);
    assert_eq!(
        state.deployments[0].profile.fingerprint,
        original.profile.fingerprint
    );
}

#[tokio::test]
async fn gputw_provisioning_and_stale_runpod_requests_make_no_cloud_calls_or_intent() {
    let f = support::Fixture::new();
    f.authorize_mock();
    f.core.select_provider(ProviderId::Gputw).await.unwrap();
    let mut request = f.request();
    assert!(f.core.provision(request.clone()).await.is_err());
    request.provider = ProviderId::Gputw;
    assert_eq!(
        f.core.provision(request).await.unwrap_err().code,
        "provider_not_ready"
    );
    assert!(f.core.hardware_for(ProviderId::Runpod).await.is_err());
    assert_eq!(
        f.core
            .hardware_for(ProviderId::Gputw)
            .await
            .err()
            .unwrap()
            .code,
        "provider_not_ready"
    );
    assert_eq!(f.provider.calls.load(Ordering::SeqCst), 0);
    assert!(f.core.store.read().unwrap().deployments.is_empty());
    assert!(f.credentials.is_empty());
}

#[tokio::test]
async fn unsupported_saved_provider_never_falls_back_to_runpod_for_routing_or_deletion() {
    let f = support::Fixture::new();
    let id = f.ready().await;
    f.core
        .store
        .update(|s| {
            s.deployments[0].provider = ProviderId::Gputw;
            Ok(())
        })
        .unwrap();
    let before = f.provider.calls.load(Ordering::SeqCst);
    assert!(endpoint(&f.core.deployment(&id).unwrap(), 8000).is_err());
    assert_eq!(
        f.core.enable(&id).await.unwrap_err().code,
        "provider_not_ready"
    );
    assert_eq!(
        f.core.finish(&id).await.unwrap_err().code,
        "provider_not_ready"
    );
    assert_eq!(f.provider.calls.load(Ordering::SeqCst), before);
    assert_eq!(f.provider.deletes.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn key_validation_and_keychain_errors_are_not_reported_as_connected() {
    let f = support::Fixture::new();
    for key in [
        "short",
        "fixture-key with-space",
        "fixture-key-with-\u{0}control",
    ] {
        assert!(f
            .core
            .save_provider_key(ProviderId::Gputw, key)
            .await
            .is_err());
    }
    assert!(f.credentials.is_empty());
    assert_eq!(
        f.core
            .check_provider_connection(ProviderId::Gputw)
            .await
            .unwrap_err()
            .code,
        "missing_gputw_key"
    );
    f.credentials.unavailable.store(true, Ordering::SeqCst);
    assert!(f
        .core
        .snapshot()
        .unwrap()
        .providers
        .iter()
        .all(|p| p.key_configured.is_none() && p.credential_store_error.is_some()));
}

#[tokio::test]
async fn read_only_setup_checks_can_run_while_deployment_cleanup_holds_the_operations_lock() {
    let f = support::Fixture::new();
    let _cleanup = f.core.operations.lock().await;
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        f.core
            .check_provider_connection(ProviderId::Runpod)
            .await
            .unwrap();
        assert_eq!(
            f.core
                .hardware_for(ProviderId::Runpod)
                .await
                .unwrap()
                .provider,
            ProviderId::Runpod
        );
    })
    .await
    .expect("read-only setup must not wait behind the cleanup lock");
    assert_eq!(f.provider.creates.load(Ordering::SeqCst), 0);
    assert_eq!(f.provider.deletes.load(Ordering::SeqCst), 0);
}
