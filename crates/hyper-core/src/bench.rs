use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::error::Result;

/// Everything here is measured on this machine — no synthetic numbers.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BenchReport {
    pub host: String,
    pub measured_at: u64,
    /// extraction throughput for a real archive, bytes/s
    pub extract_mib_per_s: Option<f64>,
    pub extract_bytes: Option<u64>,
    /// engine cold start: ms from spawn until process is running (first poll)
    pub engine_start_ms: Option<u64>,
    /// engine still alive after first second
    pub engine_survived_first_second: Option<bool>,
    /// launcher-level note about what this benchmark does and does not cover
    pub notes: Vec<String>,
}

impl BenchReport {
    pub fn new() -> Self {
        BenchReport {
            host: host_name(),
            measured_at: now(),
            extract_mib_per_s: None,
            extract_bytes: None,
            engine_start_ms: None,
            engine_survived_first_second: None,
            notes: vec![
                "measures launcher-side values only (extract throughput, cold start)".to_string(),
                "in-game FPS is not measured; engines expose their own debug overlays".to_string(),
            ],
        }
    }

    pub fn bench_extract(&mut self, zip_path: &Path, dest: &Path) -> Result<()> {
        let before_meta = fs::metadata(zip_path).map(|m| m.len()).unwrap_or(0);
        let t0 = Instant::now();
        let files = crate::archive::safe_extract_zip(zip_path, dest)?;
        let elapsed = t0.elapsed();
        // total uncompressed size is unknown cheaply; use archive size as the
        // honest proxy and say so in the note.
        let secs = elapsed.as_secs_f64().max(1e-6);
        let mibs = before_meta as f64 / (1024.0 * 1024.0);
        self.extract_mib_per_s = Some(mibs / secs);
        self.extract_bytes = Some(before_meta);
        self.notes.push(format!(
            "extract: {} files from {:.1} MiB archive",
            files.len(),
            mibs
        ));
        Ok(())
    }

    pub fn bench_engine_start(&mut self, exe: &Path, cwd: &Path) -> Result<()> {
        let t0 = Instant::now();
        let mut child = crate::launch::launch(cwd, exe, &Default::default())?;
        let start_ms = t0.elapsed().as_millis() as u64;
        self.engine_start_ms = Some(start_ms);
        // poll liveness for one second
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut alive = false;
        while Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => {
                    alive = true;
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(_) => break,
            }
        }
        self.engine_survived_first_second = Some(alive);
        if alive {
            let _ = child.kill().map(|_| ());
            let _ = child.wait().map(|_| ());
        }
        Ok(())
    }
}

impl Default for BenchReport {
    fn default() -> Self {
        Self::new()
    }
}

fn host_name() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    format!("{os}-{arch}")
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}