use crate::{
    error::{AppError, Result},
    types::*,
};
use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

pub struct StateStore {
    directory: PathBuf,
    state: Mutex<LocalState>,
    _lock: File,
}
impl StateStore {
    pub fn open(directory: PathBuf) -> Result<Self> {
        private_directory(&directory)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join("app.lock"))
            .map_err(|_| AppError::storage())?;
        lock.try_lock_exclusive().map_err(|_| {
            AppError::new(
                "already_running",
                "AI Deck is already using this state directory.",
                "Use the existing application window.",
            )
        })?;
        let path = directory.join("state.json");
        let mut state: LocalState = if path.exists() {
            serde_json::from_slice(&fs::read(&path).map_err(|_| AppError::storage())?).map_err(
                |_| {
                    AppError::new(
                        "state_corrupt",
                        "Saved state could not be decoded. It has not been overwritten.",
                        "Restore a verified backup or inspect the app-data file before retrying.",
                    )
                },
            )?
        } else {
            LocalState::default()
        };
        if state.schema_version != STATE_VERSION {
            return Err(AppError::new(
                "state_version",
                "This app cannot open the saved state version.",
                "Use the app version that created it. No data has been changed.",
            ));
        }
        for deployment in &mut state.deployments {
            if deployment.startup_completed.is_none()
                && (deployment.verified_at.is_some()
                    || deployment.stage == DeploymentStage::Ready
                    || state
                        .sessions
                        .iter()
                        .any(|s| s.deployment_id == deployment.id))
            {
                deployment.startup_completed = Some(true);
            }
            deployment.verified_at = None;
        }
        let at = now();
        let mut ended_deployments = std::collections::HashSet::new();
        for session in &mut state.sessions {
            if matches!(
                session.status,
                SessionStatus::Running | SessionStatus::Starting | SessionStatus::Disconnected
            ) {
                session.status = SessionStatus::Ended;
                session.ended_at = Some(at);
                ended_deployments.insert(session.deployment_id.clone());
            }
        }
        for id in ended_deployments {
            state.arm_idle_cleanup(&id, at);
        }
        let store = Self {
            directory,
            state: Mutex::new(state),
            _lock: lock,
        };
        store.update(|_| Ok(()))?;
        Ok(store)
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn read(&self) -> Result<LocalState> {
        self.state
            .lock()
            .map(|s| s.clone())
            .map_err(|_| AppError::storage())
    }
    pub fn update<T>(&self, apply: impl FnOnce(&mut LocalState) -> Result<T>) -> Result<T> {
        let mut locked = self.state.lock().map_err(|_| AppError::storage())?;
        let mut candidate = locked.clone();
        let result = apply(&mut candidate)?;
        let bytes = serde_json::to_vec_pretty(&candidate).map_err(|_| AppError::storage())?;
        atomic_write(&self.directory.join("state.json"), &bytes)?;
        *locked = candidate;
        Ok(result)
    }
}

pub fn private_directory(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|_| AppError::storage())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| AppError::storage())?;
    }
    Ok(())
}
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or_else(AppError::storage)?;
    let temporary = parent.join(format!(".write-{}", uuid::Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options.open(&temporary).map_err(|_| AppError::storage())?;
        file.write_all(data)
            .and_then(|_| file.sync_all())
            .map_err(|_| AppError::storage())?;
        fs::rename(&temporary, path).map_err(|_| AppError::storage())?;
        File::open(parent)
            .and_then(|d| d.sync_all())
            .map_err(|_| AppError::storage())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
