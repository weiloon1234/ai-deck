pub mod claude_adapter;
pub mod codex_adapter;

use crate::{
    error::{AppError, Result},
    types::*,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub struct LaunchSpec {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub directory: PathBuf,
    pub redacted_values: Vec<String>,
}

pub fn controlled_environment() -> BTreeMap<String, String> {
    // Allowlist, rather than a growing denylist of OAuth/provider/management keys.
    let mut env: BTreeMap<_, _> = [
        "PATH", "HOME", "USER", "LOGNAME", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE", "SHELL",
    ]
    .into_iter()
    .filter_map(|key| std::env::var(key).ok().map(|value| (key.to_owned(), value)))
    .collect();
    env.insert("TERM".into(), "xterm-256color".into());
    env.insert("COLORTERM".into(), "truecolor".into());
    env
}

pub fn discover(cli: CliKind, configured: Option<&str>) -> CliInstallation {
    let name = match cli {
        CliKind::Codex => "codex",
        CliKind::Claude => "claude",
    };
    let mut candidates: Vec<PathBuf> = configured.map(PathBuf::from).into_iter().collect();
    if configured.is_none() {
        if let Some(path) = std::env::var_os("PATH") {
            candidates.extend(std::env::split_paths(&path).map(|p| p.join(name)));
        }
        for base in ["/opt/homebrew/bin", "/usr/local/bin"] {
            candidates.push(Path::new(base).join(name));
        }
        if let Some(home) = std::env::var_os("HOME") {
            candidates.push(Path::new(&home).join(".local/bin").join(name));
        }
        if cli == CliKind::Codex {
            candidates.push(PathBuf::from("/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex"));
        }
    }
    for path in candidates {
        let Ok(path) = executable_path(&path) else {
            continue;
        };
        let result = version(&path);
        return match result {
            Ok(version) => CliInstallation {
                cli,
                path: Some(path.to_string_lossy().into()),
                version: Some(version),
                message: "Installed; nothing was upgraded.".into(),
            },
            Err(error) => CliInstallation {
                cli,
                path: Some(path.to_string_lossy().into()),
                version: None,
                message: error.message,
            },
        };
    }
    CliInstallation {
        cli,
        path: None,
        version: None,
        message: "Not found. Select an existing CLI executable in Setup.".into(),
    }
}
pub fn executable_path(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() || !path.is_file() {
        return Err(AppError::invalid(
            "Choose an existing absolute executable path.",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if path
            .metadata()
            .map_err(|_| AppError::invalid("Executable could not be read."))?
            .permissions()
            .mode()
            & 0o111
            == 0
        {
            return Err(AppError::invalid("The selected file is not executable."));
        }
    }
    path.canonicalize()
        .map_err(|_| AppError::invalid("Executable path could not be resolved."))
}
pub fn version(path: &Path) -> Result<String> {
    let mut process = Command::new(path)
        .arg("--version")
        .env_clear()
        .envs(controlled_environment())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| AppError::invalid("The CLI executable could not start."))?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match process.try_wait() {
            Ok(Some(status)) if status.success() => {
                let output = process
                    .wait_with_output()
                    .map_err(|_| AppError::invalid("Could not read CLI version."))?;
                let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                if version.len() > 200 || version.is_empty() {
                    return Err(AppError::invalid("Unexpected CLI version response."));
                }
                return Ok(version);
            }
            Ok(Some(_)) | Err(_) => return Err(AppError::invalid("The CLI version check failed.")),
            Ok(None) if Instant::now() >= deadline => {
                let _ = process.kill();
                let _ = process.wait();
                return Err(AppError::invalid("CLI version check timed out."));
            }
            _ => std::thread::sleep(Duration::from_millis(30)),
        }
    }
}

pub fn assert_known_launcher(cli: &CliKind, version: &str) -> Result<()> {
    let supported = match cli {
        CliKind::Codex => version == "codex-cli 0.160.0",
        CliKind::Claude => version == "2.1.289 (Claude Code)",
    };
    if !supported {
        return Err(AppError::new("cli_version_unverified", "This CLI version has not been checked for the adapter's isolation flags.", "Use the documented version or validate and update the adapter. The app will not downgrade isolation or install another version automatically."));
    }
    Ok(())
}
