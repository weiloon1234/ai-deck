//! macOS process metadata only: no command lines or environments are read.
//! Unique process/parent identities survive setsid and reparenting. Keep observed
//! identities after exit so a later scan can still recognize their children.
use crate::error::{AppError, Result};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy)]
struct Process {
    pid: i32,
    identity: u64,
    parent: u64,
    exited: bool,
}

pub struct ProcessTree {
    known: HashSet<u64>,
    members: HashMap<u64, Process>,
}

impl ProcessTree {
    pub fn check_available() -> Result<()> {
        platform::read(std::process::id() as i32)?.ok_or_else(process_error)?;
        platform::snapshot()?;
        Ok(())
    }
    pub fn capture(pid: u32) -> Result<Self> {
        let root = platform::read(pid as i32)?.ok_or_else(process_error)?;
        Ok(Self {
            known: HashSet::from([root.identity]),
            members: HashMap::from([(root.identity, root)]),
        })
    }

    fn discover(&mut self, processes: &[Process]) {
        loop {
            let previous = self.known.len();
            for p in processes {
                if self.known.contains(&p.identity) || self.known.contains(&p.parent) {
                    self.known.insert(p.identity);
                    self.members.insert(p.identity, *p);
                }
            }
            if self.known.len() == previous {
                break;
            }
        }
    }

    pub fn refresh(&mut self) -> Result<bool> {
        self.discover(&platform::snapshot()?);
        let mut active = HashMap::new();
        for (&identity, previous) in &self.members {
            // A vanished process is fine. Unreadable owned state is uncertainty,
            // not proof of cleanup. A reused PID must never become a member.
            if let Some(p) = platform::read(previous.pid)? {
                if p.identity == identity && !p.exited {
                    active.insert(identity, p);
                }
            }
        }
        self.members = active;
        Ok(!self.members.is_empty())
    }

    pub fn terminate(&self, force: bool) -> Result<()> {
        let mut first_error = None;
        for p in self.members.values() {
            let result = match platform::read(p.pid) {
                Ok(Some(current)) if current.identity == p.identity && !current.exited => {
                    platform::terminate(p.pid, force)
                }
                Ok(_) => Ok(()),
                Err(error) => Err(error),
            };
            if let Err(error) = result {
                first_error.get_or_insert(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

pub fn process_error() -> AppError {
    AppError::new("process_cleanup", "Local tool cleanup could not be confirmed.", "Keep the app open and retry closing the session. Inspect remaining local tools before quitting.")
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use std::mem::{size_of, MaybeUninit};

    // Apple XNU proc_info_private.h declares this stable 56-byte API layout and
    // PROC_PIDT_BSDINFOWITHUNIQID (18). libc provides the public BSD portion.
    // https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/proc_info_private.h
    #[repr(C)]
    struct UniqueInfo {
        executable_uuid: [u8; 16],
        identity: u64,
        parent: u64,
        version: i32,
        parent_version: i32,
        reserved: [u64; 2],
    }
    const _: () = assert!(size_of::<UniqueInfo>() == 56);
    #[repr(C)]
    struct Info {
        bsd: libc::proc_bsdinfo,
        unique: UniqueInfo,
    }
    pub fn read(pid: i32) -> Result<Option<Process>> {
        let mut info = MaybeUninit::<Info>::zeroed();
        // arg=1 includes unreaped zombies, so fast CLI exits can be identified.
        let copied = unsafe {
            libc::proc_pidinfo(
                pid,
                18,
                1,
                info.as_mut_ptr().cast(),
                size_of::<Info>() as i32,
            )
        };
        if copied != size_of::<Info>() as i32 {
            return if std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
                Ok(None)
            } else {
                Err(process_error())
            };
        }
        let info = unsafe { info.assume_init() };
        Ok(Some(Process {
            pid,
            identity: info.unique.identity,
            parent: info.unique.parent,
            exited: info.bsd.pbi_status == libc::SZOMB,
        }))
    }
    pub fn snapshot() -> Result<Vec<Process>> {
        // PROC_UID_ONLY=4 from the macOS SDK's sys/proc_info.h.
        let uid = unsafe { libc::geteuid() };
        let mut capacity = 1024;
        loop {
            let mut pids = vec![0i32; capacity];
            let bytes = unsafe {
                libc::proc_listpids(
                    4,
                    uid,
                    pids.as_mut_ptr().cast(),
                    (pids.len() * size_of::<i32>()) as i32,
                )
            };
            if bytes <= 0 {
                return Err(process_error());
            }
            let count = bytes as usize / size_of::<i32>();
            if count < capacity {
                // Some unrelated same-user processes can restrict inspection.
                // Known owned processes are rechecked strictly by refresh().
                return Ok(pids[..count]
                    .iter()
                    .filter(|&&pid| pid > 0)
                    .filter_map(|&pid| read(pid).ok().flatten())
                    .collect());
            }
            if capacity >= 131072 {
                return Err(process_error());
            }
            capacity *= 2;
        }
    }
    pub fn terminate(pid: i32, force: bool) -> Result<()> {
        let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
        if unsafe { libc::kill(pid, signal) } == 0
            || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
        {
            Ok(())
        } else {
            Err(process_error())
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::*;
    pub fn read(_: i32) -> Result<Option<Process>> {
        Err(process_error())
    }
    pub fn snapshot() -> Result<Vec<Process>> {
        Err(process_error())
    }
    pub fn terminate(_: i32, _: bool) -> Result<()> {
        Err(process_error())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ancestry_uses_unique_identity_not_reused_pids_or_current_parent_pid() {
        let root = Process {
            pid: 10,
            identity: 100,
            parent: 1,
            exited: false,
        };
        let mut tree = ProcessTree {
            known: HashSet::from([100]),
            members: HashMap::from([(100, root)]),
        };
        tree.discover(&[
            Process {
                pid: 12,
                identity: 102,
                parent: 101,
                exited: false,
            },
            Process {
                pid: 11,
                identity: 101,
                parent: 100,
                exited: false,
            },
            Process {
                pid: 10,
                identity: 900,
                parent: 1,
                exited: false,
            },
            Process {
                pid: 13,
                identity: 901,
                parent: 900,
                exited: false,
            },
        ]);
        assert!(tree.known.contains(&102));
        assert!(!tree.known.contains(&900));
        assert!(!tree.known.contains(&901));
        // The parent can already be gone or reparented when its child is seen.
        tree.members.clear();
        tree.discover(&[Process {
            pid: 14,
            identity: 103,
            parent: 102,
            exited: false,
        }]);
        assert!(tree.known.contains(&103));
    }
}
