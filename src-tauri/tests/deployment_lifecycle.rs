mod support;
use ai_deck::{
    deployment_service::validate_policy, model_catalog::builtin_catalog, state_store::StateStore,
    types::*,
};
use std::sync::atomic::Ordering;
use support::*;

#[tokio::test]
async fn transient_verification_blocks_routing_without_deleting_and_then_recovers() {
    for code in ["rate_limit", "provider_unavailable", "network_unavailable"] {
        let f = Fixture::new();
        let id = f.ready().await;
        f.core.enable(&id).await.unwrap();
        f.core
            .store
            .update(|s| {
                s.deployments[0].verified_at = None;
                Ok(())
            })
            .unwrap();
        *f.runtime.verification_error.lock().unwrap() = Some(code.into());
        f.core.reconcile_all().await.unwrap();
        let d = f.core.deployment(&id).unwrap();
        assert_eq!(d.stage, DeploymentStage::ReconciliationRequired);
        assert!(d.verified_at.is_none());
        assert!(f.core.enable(&id).await.is_err());
        assert_eq!(f.provider.deletes.load(Ordering::SeqCst), 0);
        *f.runtime.verification_error.lock().unwrap() = None;
        f.core.reconcile_all().await.unwrap();
        assert_eq!(
            f.core.deployment(&id).unwrap().stage,
            DeploymentStage::Ready
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
    }
}

#[tokio::test]
async fn initial_startup_is_bounded_but_old_ready_or_unknown_records_survive_outages() {
    for completed in [Some(false), Some(true), None] {
        let f = Fixture::new();
        let id = f.ready().await;
        f.core
            .store
            .update(|s| {
                let d = &mut s.deployments[0];
                d.startup_completed = completed;
                d.created_at = now()
                    - d.profile.runtime.download_timeout_seconds
                    - d.profile.runtime.load_timeout_seconds
                    - 301;
                d.deadline_at = now() + 1800;
                d.budget_usd = 100.0;
                Ok(())
            })
            .unwrap();
        f.runtime.status_offline.store(true, Ordering::SeqCst);
        f.core.reconcile_all().await.unwrap();
        assert_eq!(
            f.provider.deletes.load(Ordering::SeqCst),
            usize::from(completed == Some(false))
        );
        if completed != Some(false) {
            assert_eq!(
                f.core.deployment(&id).unwrap().stage,
                DeploymentStage::ReconciliationRequired
            );
            f.runtime.status_offline.store(false, Ordering::SeqCst);
            f.core.reconcile_all().await.unwrap();
            assert_eq!(
                f.core.deployment(&id).unwrap().stage,
                DeploymentStage::Ready
            );
            f.core
                .store
                .update(|s| {
                    s.deployments[0].deadline_at = now() - 1;
                    Ok(())
                })
                .unwrap();
            f.core.reconcile_all().await.unwrap();
            assert_eq!(
                f.core.deployment(&id).unwrap().stage,
                DeploymentStage::Terminated
            );
        }
    }
}

#[tokio::test]
async fn inadequate_retained_cache_is_rejected_before_creating_a_pod() {
    let f = Fixture::new();
    f.authorize_mock();
    *f.provider.volume_size.lock().unwrap() = 1;
    let mut request = f.request();
    request.network_volume_id = Some("retained-volume".into());
    assert_eq!(
        f.core.provision(request.clone()).await.unwrap_err().code,
        "insufficient_cache_space"
    );
    assert_eq!(f.provider.creates.load(Ordering::SeqCst), 0);
    assert!(f.core.store.read().unwrap().deployments.is_empty());
    *f.provider.volume_size.lock().unwrap() = builtin_catalog().unwrap()[0].model.min_cache_gb;
    f.core.provision(request).await.unwrap();
    let payload = f.provider.payloads.lock().unwrap()[0].clone();
    let config: serde_json::Value =
        serde_json::from_str(payload["env"]["RUNPOD_DECK_RUNTIME"].as_str().unwrap()).unwrap();
    assert_eq!(
        config["minCacheGb"],
        builtin_catalog().unwrap()[0].model.min_cache_gb
    );
}

#[tokio::test]
async fn damaged_catalog_or_keychain_keeps_saved_state_and_cleanup_accessible() {
    let f = Fixture::new();
    let id = f.ready().await;
    f.credentials.unavailable.store(true, Ordering::SeqCst);
    std::fs::write(f.core.store.directory().join("catalog.json"), "broken-json").unwrap();
    let snapshot = f.core.snapshot().unwrap();
    assert_eq!(snapshot.state.deployments[0].id, id);
    assert!(snapshot.catalog.is_empty());
    assert!(snapshot.catalog_error.is_some());
    assert!(snapshot.runpod_key_configured.is_none());
    assert_eq!(
        snapshot.credential_store_error.unwrap().code,
        "credential_store"
    );
    f.credentials.unavailable.store(false, Ordering::SeqCst);
    f.core.finish(&id).await.unwrap();
    assert_eq!(
        f.core.snapshot().unwrap().state.deployments[0].stage,
        DeploymentStage::Terminated
    );
    std::fs::remove_file(f.core.store.directory().join("catalog.json")).unwrap();
    let snapshot = f.core.snapshot().unwrap();
    assert!(snapshot.catalog_error.is_none() && snapshot.credential_store_error.is_none());
    assert!(!snapshot.catalog.is_empty());
}

#[tokio::test]
async fn paid_actions_default_to_disabled_and_never_reach_provider() {
    let f = Fixture::new();
    assert_eq!(
        f.core.provision(f.request()).await.unwrap_err().code,
        "paid_actions_disabled"
    );
    assert_eq!(f.provider.calls.load(Ordering::SeqCst), 0);
    assert!(f.core.store.read().unwrap().deployments.is_empty());
    f.authorize_mock();
    let mut request = f.request();
    request.acknowledge_paid_creation = false;
    assert!(f.core.provision(request).await.is_err());
    assert_eq!(f.provider.calls.load(Ordering::SeqCst), 0);
}
#[test]
fn spending_authorization_requires_all_limits() {
    assert!(validate_policy(&Policy::default()).is_ok());
    let mut p = Policy {
        paid_provisioning_enabled: true,
        ..Policy::default()
    };
    assert!(validate_policy(&p).is_err());
    p.total_budget_usd = Some(10.0);
    p.max_lifetime_minutes = Some(20);
    p.acknowledge_offline_risk = true;
    assert!(validate_policy(&p).is_ok());
    p.max_hourly_usd = 10.01;
    assert!(validate_policy(&p).is_err());
    p.max_hourly_usd = f64::NAN;
    assert!(validate_policy(&p).is_err());
}
#[tokio::test]
async fn provision_ready_enable_and_finish_are_separate_and_repeatable() {
    let f = Fixture::new();
    let id = f.ready().await;
    assert!(f.core.store.read().unwrap().enabled_deployment_id.is_none());
    f.core.enable(&id).await.unwrap();
    assert_eq!(
        f.core
            .store
            .read()
            .unwrap()
            .enabled_deployment_id
            .as_deref(),
        Some(id.as_str())
    );
    assert_eq!(f.provider.creates.load(Ordering::SeqCst), 1);
    f.core.finish(&id).await.unwrap();
    f.core.finish(&id).await.unwrap();
    let d = f.core.deployment(&id).unwrap();
    assert_eq!(d.stage, DeploymentStage::Terminated);
    assert!(d.terminated_at.is_some());
    assert!(f.core.store.read().unwrap().enabled_deployment_id.is_none());
    assert_eq!(f.provider.deletes.load(Ordering::SeqCst), 1);
    assert!(f.core.enable(&id).await.is_err());
}
#[tokio::test]
async fn lost_creation_is_reconciled_without_a_second_create_and_checks_actual_price() {
    let f = Fixture::new();
    f.authorize_mock();
    f.provider.lose_create.store(true, Ordering::SeqCst);
    *f.provider.actual_price.lock().unwrap() = 11.0;
    assert_eq!(
        f.core.provision(f.request()).await.unwrap_err().code,
        "ambiguous_create"
    );
    assert!(f.core.provision(f.request()).await.is_err());
    f.core.reconcile_all().await.unwrap();
    let d = f.core.store.read().unwrap().deployments.remove(0);
    assert_eq!(d.stage, DeploymentStage::Terminated);
    assert_eq!(d.hourly_usd, 11.0);
    assert!(d.failure_reason.unwrap().contains("price"));
    assert_eq!(f.provider.creates.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn lost_delete_response_is_not_proof_of_failure_or_success() {
    let f = Fixture::new();
    let id = f.ready().await;
    f.provider.lose_delete.store(true, Ordering::SeqCst);
    f.provider.keep_after_delete.store(true, Ordering::SeqCst);
    f.core.finish(&id).await.unwrap();
    assert_eq!(
        f.core.deployment(&id).unwrap().stage,
        DeploymentStage::CleanupPending
    );
    assert!(f.core.deployment(&id).unwrap().terminated_at.is_none());
    f.provider.keep_after_delete.store(false, Ordering::SeqCst);
    f.core.reconcile_all().await.unwrap();
    assert_eq!(
        f.core.deployment(&id).unwrap().stage,
        DeploymentStage::Terminated
    );
}
#[tokio::test]
async fn cleanup_refuses_a_pod_whose_ownership_changed() {
    let f = Fixture::new();
    let id = f.ready().await;
    f.provider.pods.lock().unwrap()[0]
        .env
        .insert("RUNPOD_DECK_OWNER".into(), "someone-else".into());
    assert_eq!(
        f.core.finish(&id).await.unwrap_err().code,
        "ownership_mismatch"
    );
    assert_eq!(f.provider.deletes.load(Ordering::SeqCst), 0);
    assert_eq!(
        f.core.deployment(&id).unwrap().stage,
        DeploymentStage::CleanupPending
    );
}
#[tokio::test]
async fn retained_cache_and_runtime_payload_never_receive_management_credentials() {
    let f = Fixture::new();
    f.authorize_mock();
    let mut request = f.request();
    request.network_volume_id = Some("retained-volume".into());
    let id = f.core.provision(request).await.unwrap();
    let payload = f.provider.payloads.lock().unwrap()[0].clone();
    assert_eq!(payload["networkVolumeId"], "retained-volume");
    assert!(payload["dockerEntrypoint"].is_array() && payload["dockerStartCmd"].is_array());
    let env = payload["env"].as_object().unwrap();
    assert!(!env.contains_key("RUNPOD_API_KEY"));
    assert!(!env.contains_key("HF_TOKEN"));
    assert!(env["VLLM_API_KEY"].as_str().unwrap().starts_with("rpd_"));
    f.core.finish(&id).await.unwrap();
    assert_eq!(
        f.core.deployment(&id).unwrap().network_volume_id.as_deref(),
        Some("retained-volume")
    );
    assert_eq!(f.provider.deletes.load(Ordering::SeqCst), 1);
    // RunpodApi intentionally has no network-volume deletion operation.
}
#[tokio::test]
async fn failed_inference_blocks_enable_and_preserves_original_failure_after_cleanup() {
    let f = Fixture::new();
    f.authorize_mock();
    f.runtime.fail.store(true, Ordering::SeqCst);
    let id = f.core.provision(f.request()).await.unwrap();
    f.core.reconcile_all().await.unwrap();
    assert_eq!(
        f.core.deployment(&id).unwrap().stage,
        DeploymentStage::Terminated
    );
    assert!(f.core.enable(&id).await.is_err());
    assert!(f.core.store.read().unwrap().enabled_deployment_id.is_none());
    f.core.finish(&id).await.unwrap();
    assert!(f
        .core
        .deployment(&id)
        .unwrap()
        .failure_reason
        .unwrap()
        .contains("wrong model"));
}
#[tokio::test]
async fn runtime_failure_and_expired_deadline_request_verified_cleanup() {
    let f = Fixture::new();
    let id = f.ready().await;
    *f.runtime.stage.lock().unwrap() = "failed".into();
    *f.runtime.code.lock().unwrap() = "model_access_denied".into();
    f.core.reconcile_all().await.unwrap();
    assert_eq!(
        f.core.deployment(&id).unwrap().stage,
        DeploymentStage::Terminated
    );
    assert!(f
        .core
        .deployment(&id)
        .unwrap()
        .failure_reason
        .unwrap()
        .contains("access was denied"));
    *f.runtime.stage.lock().unwrap() = "verifying".into();
    let id = f.core.provision(f.request()).await.unwrap();
    f.core
        .store
        .update(|s| {
            s.deployments.last_mut().unwrap().deadline_at = now() - 1;
            Ok(())
        })
        .unwrap();
    f.core.reconcile_all().await.unwrap();
    assert_eq!(
        f.core.deployment(&id).unwrap().stage,
        DeploymentStage::Terminated
    );
}
#[tokio::test]
async fn provider_disappearance_invalidates_route_and_offline_never_confirms_termination() {
    let f = Fixture::new();
    let id = f.ready().await;
    f.core.enable(&id).await.unwrap();
    f.provider.offline.store(true, Ordering::SeqCst);
    assert!(f.core.reconcile_all().await.is_err());
    assert!(f.core.deployment(&id).unwrap().verified_at.is_none());
    assert!(f.core.finish(&id).await.is_err());
    assert!(f.core.deployment(&id).unwrap().terminated_at.is_none());
    f.provider.offline.store(false, Ordering::SeqCst);
    f.provider.pods.lock().unwrap().clear();
    f.core.reconcile_all().await.unwrap();
    assert_eq!(
        f.core.deployment(&id).unwrap().stage,
        DeploymentStage::Terminated
    );
    assert!(f.core.store.read().unwrap().enabled_deployment_id.is_none());
}
#[tokio::test]
async fn restart_revalidates_saved_route_and_preserves_profile_snapshot() {
    let f = Fixture::new();
    let id = f.ready().await;
    f.core.enable(&id).await.unwrap();
    let path = f.core.store.directory().to_path_buf();
    let initial = f.core.deployment(&id).unwrap();
    let mut catalog: serde_json::Value =
        serde_json::from_str(include_str!("../../model-catalog/catalog.json")).unwrap();
    catalog["profiles"][0]["contextTokens"] = 4096.into();
    std::fs::write(path.join("catalog.json"), catalog.to_string()).unwrap();
    assert_ne!(
        f.core.catalog().unwrap()[0].fingerprint,
        initial.profile.fingerprint
    );
    assert_eq!(
        f.core.deployment(&id).unwrap().profile.fingerprint,
        builtin_catalog().unwrap()[0].fingerprint
    );
    drop(f.core);
    drop(f.provider);
    let restarted = StateStore::open(path).unwrap();
    assert_eq!(
        restarted.read().unwrap().enabled_deployment_id.as_deref(),
        Some(id.as_str())
    );
    assert!(restarted.read().unwrap().deployments[0]
        .verified_at
        .is_none());
}
#[tokio::test]
async fn late_creation_after_confirmed_absence_is_found_and_cleaned_up() {
    let f = Fixture::new();
    f.authorize_mock();
    f.provider.lose_create.store(true, Ordering::SeqCst);
    assert!(f.core.provision(f.request()).await.is_err());
    let pod = f.provider.pods.lock().unwrap().pop().unwrap();
    f.core
        .store
        .update(|s| {
            s.deployments[0].created_at = now() - 125;
            Ok(())
        })
        .unwrap();
    let d = f.core.store.read().unwrap().deployments.remove(0);
    f.core
        .confirm_absent_creation(&d.id, &d.pod_name)
        .await
        .unwrap();
    f.provider.pods.lock().unwrap().push(pod);
    f.core.reconcile_all().await.unwrap();
    assert_eq!(f.provider.deletes.load(Ordering::SeqCst), 1);
    assert_eq!(
        f.core.deployment(&d.id).unwrap().stage,
        DeploymentStage::Terminated
    );
}

#[tokio::test]
async fn one_pending_deletion_does_not_skip_other_owned_duplicates() {
    let f = Fixture::new();
    let id = f.ready().await;
    let mut duplicate = f.provider.pods.lock().unwrap()[0].clone();
    duplicate.id = "duplicate-pod".into();
    f.provider.pods.lock().unwrap().push(duplicate);
    f.core.reconcile_all().await.unwrap();
    assert_eq!(
        f.core.deployment(&id).unwrap().stage,
        DeploymentStage::ReconciliationRequired
    );
    f.provider.keep_after_delete.store(true, Ordering::SeqCst);
    f.core.finish(&id).await.unwrap();
    assert_eq!(f.provider.deletes.load(Ordering::SeqCst), 2);
    assert_eq!(
        f.core.deployment(&id).unwrap().stage,
        DeploymentStage::CleanupPending
    );
    f.provider.keep_after_delete.store(false, Ordering::SeqCst);
    f.core.reconcile_all().await.unwrap();
    assert_eq!(
        f.core.deployment(&id).unwrap().stage,
        DeploymentStage::Terminated
    );
}

#[tokio::test]
async fn gated_profile_payload_uses_only_a_secret_reference() {
    let f = Fixture::new();
    let id = f.ready().await;
    let mut d = f.core.deployment(&id).unwrap();
    d.profile.model.access = ai_deck::model_catalog::ModelAccess::Gated;
    assert!(
        ai_deck::deployment_service::create_payload(&d, "owner", "fixture-token", None).is_err()
    );
    let payload = ai_deck::deployment_service::create_payload(
        &d,
        "owner",
        "fixture-token",
        Some("hub_read_token"),
    )
    .unwrap();
    assert_eq!(
        payload["env"]["HF_TOKEN"],
        "{{ RUNPOD_SECRET_hub_read_token }}"
    );
    assert_eq!(payload["env"]["VLLM_API_KEY"], "fixture-token");
    assert!(!payload["env"]
        .as_object()
        .unwrap()
        .contains_key("RUNPOD_API_KEY"));
}
