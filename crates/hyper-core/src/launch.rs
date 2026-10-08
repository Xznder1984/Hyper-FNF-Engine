use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use crate::error::{HyperError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    Low,
    Normal,
    High,
    Realtime,
}

impl Priority {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "low" => Some(Priority::Low),
            "normal" => Some(Priority::Normal),
            "high" => Some(Priority::High),
            "realtime" => Some(Priority::Realtime),
            _ => None,
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Priority::parse(s).ok_or_else(|| {
            format!("unknown priority '{s}' (expected low|normal|high|realtime)")
        })
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Priority::Low => "low",
            Priority::Normal => "normal",
            Priority::High => "high",
            Priority::Realtime => "realtime",
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct LaunchOptions {
    pub priority: Option<Priority>,
    /// extra args appended after the engine's own
    pub extra_args: Vec<String>,
    /// environment overrides
    pub env: Vec<(String, String)>,
}

/// Find the engine's main executable inside its root.
pub fn find_executable(root: &Path) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        for name in ["Funkin.exe", "funkin.exe", "Funkin_win.exe"] {
            let p = root.join(name);
            if p.is_file() {
                return Some(p);
            }
        }
        // fallback: first top-level .exe
        if let Ok(rd) = fs::read_dir(root) {
            let mut exes: Vec<PathBuf> = rd
                .flatten()
                .filter(|e| {
                    e.file_type().map(|t| t.is_file()).unwrap_or(false)
                        && e.file_name().to_string_lossy().to_lowercase().ends_with(".exe")
                })
                .map(|e| e.path())
                .collect();
            exes.sort();
            return exes.into_iter().next();
        }
        #[allow(unreachable_code)]
        {
            None
        }
    }
    #[cfg(unix)]
    {
        for name in ["Funkin", "funkin", "Funkin-linux"] {
            let p = root.join(name);
            if p.is_file() && is_executable(&p) {
                return Some(p);
            }
        }
        if let Ok(rd) = fs::read_dir(root) {
            for e in rd.flatten() {
                let p = e.path();
                if e.file_type().map(|t| t.is_file()).unwrap_or(false)
                    && is_executable(&p)
                    && !p.extension().map(|x| x == "so").unwrap_or(false)
                {
                    return Some(p);
                }
            }
        }
        #[allow(unreachable_code)]
        {
            None
        }
    }
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(p).map(|m| m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}

/// Stage a mod folder into the engine's mods dir so the engine sees it.
/// Returns the final mod folder name used.
pub fn stage_mod(engine_root: &Path, def_mods_dir: &str, mod_dir: &Path) -> Result<String> {
    let mods = engine_root.join(def_mods_dir);
    fs::create_dir_all(&mods)?;
    let name = mod_folder_name(mod_dir);
    let target = mods.join(&name);
    if target.exists() {
        // already staged: refresh contents from source (keeps modsList order)
        fs::remove_dir_all(&target)?;
    }
    copy_tree(mod_dir, &target)?;
    Ok(name)
}

pub fn unstage_mod(engine_root: &Path, def_mods_dir: &str, name: &str) -> Result<()> {
    let target = engine_root.join(def_mods_dir).join(name);
    if target.exists() {
        fs::remove_dir_all(&target)?;
    }
    Ok(())
}

/// Sanitise a mod folder name for staging (keeps it readable + path-safe).
pub fn mod_folder_name(mod_dir: &Path) -> String {
    let base = mod_dir.file_name().map(|n| n.to_string_lossy().to_string());
    let clean: String = base
        .map(|b| {
            b.chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let trimmed = clean.trim().to_string();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        // fall back to a stable hash-based name
        crate::detect::mod_key(mod_dir).unwrap_or_else(|_| "mod-unknown".into())
    } else {
        trimmed
    }
}

/// Launch the engine with the staged mod.
pub fn launch(
    engine_root: &Path,
    exe: &Path,
    opts: &LaunchOptions,
) -> Result<Child> {
    let mut cmd = Command::new(exe);
    cmd.current_dir(engine_root);
    for (k, v) in &opts.env {
        cmd.env(k, v);
    }
    cmd.args(&opts.extra_args);
    let child = cmd.spawn().map_err(|e| {
        HyperError::Message(format!(
            "could not start {}: {e}",
            exe.display()
        ))
    })?;
    if let Some(p) = opts.priority {
        set_priority(child.id(), p)?;
    }
    Ok(child)
}

fn set_priority(pid: u32, priority: Priority) -> Result<()> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
        use windows_sys::Win32::System::Threading::{
            OpenProcess, SetPriorityClass, BELOW_NORMAL_PRIORITY_CLASS,
            NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS, REALTIME_PRIORITY_CLASS,
            PROCESS_SET_INFORMATION,
        };
        unsafe {
            let handle: HANDLE = OpenProcess(PROCESS_SET_INFORMATION, 0, pid);
            if handle.is_null() {
                return Err(HyperError::Message(format!(
                    "could not open process {pid} to set priority (launcher-level option; run with the same user)"
                )));
            }
            let class = match priority {
                Priority::Low => BELOW_NORMAL_PRIORITY_CLASS,
                Priority::Normal => NORMAL_PRIORITY_CLASS,
                Priority::High => HIGH_PRIORITY_CLASS,
                Priority::Realtime => REALTIME_PRIORITY_CLASS,
            };
            let ok = SetPriorityClass(handle, class);
            CloseHandle(handle);
            if ok == 0 {
                return Err(HyperError::Message(format!(
                    "SetPriorityClass failed for {pid}"
                )));
            }
        }
        Ok(())
    }
    #[cfg(unix)]
    {
        let prio = match priority {
            Priority::Low => 10i32,
            Priority::Normal => 0,
            Priority::High => -5,
            Priority::Realtime => -10,
        };
        // best-effort; unprivileged users often cannot renice other processes
        let _ = unsafe { libc::setpriority(libc::PRIO_PROCESS, pid as libc::id_t, prio) };
        Ok(())
    }
}

fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    for e in fs::read_dir(src)? {
        let e = e?;
        let to = dst.join(e.file_name());
        if e.file_type()?.is_dir() {
            fs::create_dir_all(&to)?;
            copy_tree(&e.path(), &to)?;
        } else {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(e.path(), &to)?;
        }
    }
    Ok(())
}

/// Path of the currently running launcher binary (for update/rollback).
pub fn current_exe() -> PathBuf {
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("hfe"))
}

/// Detect whether the launcher was installed by a system package manager
/// (AUR, flatpak, snap...). When true, self-update is disabled.
pub fn managed_install() -> Option<String> {
    if let Ok(v) = std::env::var("HYPER_MANAGED") {
        if !v.is_empty() {
            return Some(v);
        }
    }
    #[cfg(target_os = "linux")]
    {
        let exe = current_exe();
        let s = exe.to_string_lossy();
        if s.contains("/usr/bin/") || s.contains("/usr/local/bin/") {
            return Some("system package manager (e.g. AUR)".to_string());
        }
        if s.contains("/snap/") {
            return Some("snap".to_string());
        }
        if s.contains("/var/lib/flatpak") || s.contains("/app/") {
            return Some("flatpak".to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_mod_names() {
        assert_eq!(
            mod_folder_name(Path::new("C:/fun mods/My Cool Mod!")),
            "My Cool Mod_"
        );
        let weird = Path::new("C:/a/../..");
        let h = mod_folder_name(weird);
        assert!(h.starts_with("mod-") || h.len() >= 6, "{h}");
    }

    #[test]
    fn priority_parse() {
        assert_eq!(Priority::parse("high"), Some(Priority::High));
        assert_eq!(Priority::parse("LOW"), Some(Priority::Low));
        assert_eq!(Priority::parse("max"), None);
    }
}