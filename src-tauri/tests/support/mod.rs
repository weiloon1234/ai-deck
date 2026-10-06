#![allow(dead_code)]
use ai_deck::{
    app_core::AppCore,
    credential_store::CredentialStore,
    error::{AppError, Result},
    model_catalog::builtin_catalog,
    runpod_client::*,
    runtime_client::{RuntimeApi, RuntimeStatus},
    state_store::StateStore,
    terminal_process::TerminalManager,
    types::*,
};
use async_trait::async_trait;
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

#[derive(Default)]
pub struct MemoryCredentials {
    values: Mutex<HashMap<String, String>>,
    pub unavailable: AtomicBool,
}
impl CredentialStore for MemoryCredentials {
    fn get(&self, key: &str) -> Result<Option<String>> {
        if self.unavailable.load(Ordering::SeqCst) {
            return Err(AppError::new(
                "credential_store",
                "Fixture access denied",
                "Fixture recovery",
            ));
        }
        Ok(self.values.lock().unwrap().get(key).cloned())
    }
    fn set(&self, key: &str, value: &str) -> Result<()> {
        self.values.lock().unwrap().insert(key.into(), value.into());
        Ok(())
    }
    fn delete(&self, key: &str) -> Result<()> {
        self.values.lock().unwrap().remove(key);
        Ok(())
    }
}
pub struct Provider {
    pub pods: Mutex<Vec<Pod>>,
    pub calls: AtomicUsize,
    pub creates: AtomicUsize,
    pub deletes: AtomicUsize,
    pub lose_create: AtomicBool,
    pub keep_after_delete: AtomicBool,
    pub lose_delete: AtomicBool,
    pub offline: AtomicBool,
    pub actual_price: Mutex<f64>,
    pub volume_size: Mutex<u32>,
    pub payloads: Mutex<Vec<Value>>,
    pub store: Arc<StateStore>,
}
impl Provider {
    fn touch(&self) -> Result<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.offline.load(Ordering::SeqCst) {
            Err(AppError::network())
        } else {
            Ok(())
        }
    }
}
#[async_trait]
impl RunpodApi for Provider {
    async fn create(&self, payload: Value) -> Result<Pod> {
        self.touch()?;
        self.creates.fetch_add(1, Ordering::SeqCst);
        let state = self.store.read()?;
        // A cloud call is forbidden until its identity and intent survive a restart.
        let saved: LocalState = serde_json::from_slice(
            &std::fs::read(self.store.directory().join("state.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(saved.deployments.len(), state.deployments.len());
        let d = state.deployments.last().unwrap();
        assert_eq!(d.stage, DeploymentStage::Provisioning);
        assert_eq!(payload["env"]["RUNPOD_DECK_DEPLOYMENT"], d.id);
        assert_eq!(payload["env"]["RUNPOD_DECK_OWNER"], state.installation_id);
        let pod = Pod {
            id: format!("pod{}", self.creates.load(Ordering::SeqCst)),
            name: d.pod_name.clone(),
            image_name: d.profile.runtime.image.clone(),
            desired_status: "RUNNING".into(),
            cost_per_hr: *self.actual_price.lock().unwrap(),
            env: serde_json::from_value(payload["env"].clone()).unwrap(),
            network_volume_id: d.network_volume_id.clone(),
        };
        self.payloads.lock().unwrap().push(payload);
        self.pods.lock().unwrap().push(pod.clone());
        if self.lose_create.load(Ordering::SeqCst) {
            Err(AppError::network())
        } else {
            Ok(pod)
        }
    }
    async fn list(&self) -> Result<Vec<Pod>> {
        self.touch()?;
        Ok(self.pods.lock().unwrap().clone())
    }
    async fn get(&self, id: &str) -> Result<Option<Pod>> {
        self.touch()?;
        Ok(self
            .pods
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.id == id)
            .cloned())
    }
    async fn delete(&self, id: &str) -> Result<()> {
        self.touch()?;
        self.deletes.fetch_add(1, Ordering::SeqCst);
        if !self.keep_after_delete.load(Ordering::SeqCst) {
            self.pods.lock().unwrap().retain(|p| p.id != id);
        }
        if self.lose_delete.load(Ordering::SeqCst) {
            Err(AppError::network())
        } else {
            Ok(())
        }
    }
    async fn hardware(&self) -> Result<Hardware> {
        self.touch()?;
        let p = builtin_catalog()?.remove(0);
        Ok(Hardware {
            gpus: vec![GpuType {
                id: p.model.gpu_types[0].clone(),
                display_name: "Fixture GPU".into(),
                memory_in_gb: 80,
                secure_cloud: true,
                secure_price: Some(3.0),
            }],
            data_centers: vec![DataCenter {
                id: "fixture-dc".into(),
                name: "Fixture".into(),
                location: "Offline".into(),
                gpu_availability: vec![GpuAvailability {
                    gpu_type_id: p.model.gpu_types[0].clone(),
                    stock_status: Some("High".into()),
                }],
            }],
            network_volumes: vec![NetworkVolume {
                id: "retained-volume".into(),
                name: "Fixture cache".into(),
                size: *self.volume_size.lock().unwrap(),
                data_center_id: "fixture-dc".into(),
            }],
        })
    }
}
pub struct Runtime {
    pub fail: AtomicBool,
    pub status_offline: AtomicBool,
    pub verification_error: Mutex<Option<String>>,
    pub checks: AtomicUsize,
    pub stage: Mutex<String>,
    pub code: Mutex<String>,
}
#[async_trait]
impl RuntimeApi for Runtime {
    async fn status(&self, _: &Deployment) -> Result<RuntimeStatus> {
        if self.status_offline.load(Ordering::SeqCst) {
            return Err(AppError::network());
        }
        Ok(RuntimeStatus {
            stage: self.stage.lock().unwrap().clone(),
            code: self.code.lock().unwrap().clone(),
        })
    }
    async fn verify(&self, _: &Deployment) -> Result<()> {
        self.checks.fetch_add(1, Ordering::SeqCst);
        if let Some(code) = self.verification_error.lock().unwrap().as_ref() {
            return Err(AppError::new(
                code,
                "Fixture verification unavailable",
                "Retry",
            ));
        }
        if self.fail.load(Ordering::SeqCst) {
            Err(AppError::new(
                "wrong_model",
                "The endpoint serves the wrong model.",
                "Recreate it.",
            ))
        } else {
            Ok(())
        }
    }
}
pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub core: Arc<AppCore>,
    pub credentials: Arc<MemoryCredentials>,
    pub provider: Arc<Provider>,
    pub runtime: Arc<Runtime>,
}
impl Fixture {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(StateStore::open(directory.path().join("state")).unwrap());
        let provider = Arc::new(Provider {
            pods: Mutex::new(vec![]),
            calls: AtomicUsize::new(0),
            creates: AtomicUsize::new(0),
            deletes: AtomicUsize::new(0),
            lose_create: AtomicBool::new(false),
            keep_after_delete: AtomicBool::new(false),
            lose_delete: AtomicBool::new(false),
            offline: AtomicBool::new(false),
            actual_price: Mutex::new(3.0),
            volume_size: Mutex::new(200),
            payloads: Mutex::new(vec![]),
            store: store.clone(),
        });
        let runtime = Arc::new(Runtime {
            fail: AtomicBool::new(false),
            status_offline: AtomicBool::new(false),
            verification_error: Mutex::new(None),
            checks: AtomicUsize::new(0),
            stage: Mutex::new("verifying".into()),
            code: Mutex::new("fixture".into()),
        });
        let credentials = Arc::new(MemoryCredentials::default());
        let core = Arc::new(AppCore {
            store,
            credentials: credentials.clone(),
            provider: provider.clone(),
            runtime: runtime.clone(),
            terminals: Arc::new(TerminalManager::default()),
            operations: tokio::sync::Mutex::new(()),
            model_imports: Default::default(),
            notify: Arc::new(|_, _| {}),
        });
        Self {
            directory,
            core,
            credentials,
            provider,
            runtime,
        }
    }
    pub fn authorize_mock(&self) {
        self.core
            .store
            .update(|s| {
                s.settings.policy = Policy {
                    paid_provisioning_enabled: true,
                    total_budget_usd: Some(10.0),
                    max_lifetime_minutes: Some(60),
                    acknowledge_offline_risk: true,
                    ..Policy::default()
                };
                Ok(())
            })
            .unwrap();
    }
    pub fn request(&self) -> ProvisionRequest {
        let p = builtin_catalog().unwrap().remove(0);
        ProvisionRequest {
            profile_id: p.model.id,
            gpu_type: p.model.gpu_types[0].clone(),
            data_center_id: "fixture-dc".into(),
            network_volume_id: None,
            acknowledge_paid_creation: true,
        }
    }
    pub async fn ready(&self) -> String {
        self.authorize_mock();
        let id = self.core.provision(self.request()).await.unwrap();
        self.core.reconcile_all().await.unwrap();
        assert_eq!(
            self.core.deployment(&id).unwrap().stage,
            DeploymentStage::Ready
        );
        id
    }
}
