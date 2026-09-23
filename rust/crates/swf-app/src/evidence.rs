//! Evidence pack handling: events log, scenario results, run manifest.
//!
//! Everything written here is observational. A digest identifies an
//! artifact; it is not by itself proof of correct behavior — verdicts come
//! from the protected verifier bound to the actual artifact.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

/// One line in events-<scenario>.jsonl. `dir` marks direction relative to
/// the session: "in" (bridge -> session), "out" (session -> bridge), or
/// "session" for session-level records (timeouts, violations).
#[derive(Debug, Serialize)]
pub struct Event {
    pub ts_unix_ms: u128,
    pub dir: &'static str,
    pub scenario: String,
    pub line: String,
}

pub struct EventLog {
    writer: BufWriter<File>,
    scenario: String,
}

impl EventLog {
    pub fn create(evidence_dir: &Path, scenario: &str) -> io::Result<Self> {
        fs::create_dir_all(evidence_dir)?;
        let path = evidence_dir.join(format!("events-{scenario}.jsonl"));
        let file = File::create(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
            scenario: scenario.to_string(),
        })
    }

    pub fn record(&mut self, dir: &'static str, line: &str) -> io::Result<()> {
        let event = Event {
            ts_unix_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
            dir,
            scenario: self.scenario.clone(),
            line: line.to_string(),
        };
        serde_json::to_writer(&mut self.writer, &event)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioResult {
    pub scenario: String,
    pub backend: String,
    pub outcome: String,
    pub success: bool,
    pub task_dispatches: u32,
    pub decisions: u32,
    pub rejections: Vec<String>,
    pub ticks: u64,
    pub wall_time_ms: u128,
}

/// Append a result to scenario-results.json (a JSON array) in the evidence dir.
pub fn append_scenario_result(evidence_dir: &Path, result: &ScenarioResult) -> io::Result<()> {
    fs::create_dir_all(evidence_dir)?;
    let path = evidence_dir.join("scenario-results.json");
    let mut results: Vec<serde_json::Value> = match File::open(&path) {
        Ok(f) => serde_json::from_reader(f)?,
        Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e),
    };
    results.push(serde_json::to_value(result)?);
    let f = File::create(path)?;
    serde_json::to_writer_pretty(f, &results)?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct Manifest {
    pub run_id: String,
    pub created_unix_ms: u128,
    pub source: SourceIdentity,
    pub executable: ExecutableIdentity,
    pub simulator: SimulatorIdentity,
    pub policy_max_observation_age_ms: u64,
    pub scope: &'static str,
}

#[derive(Debug, Serialize)]
pub struct SourceIdentity {
    pub git_commit: String,
    pub git_dirty: bool,
    pub patch_applied: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExecutableIdentity {
    pub path: PathBuf,
    pub sha256: String,
}

#[derive(Debug, Serialize)]
pub struct SimulatorIdentity {
    pub backend: String,
    pub python: String,
    pub python_version: String,
    pub platform: String,
}

pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut f = File::open(path)?;
    let mut hasher = Sha256::new();
    io::copy(&mut f, &mut hasher)?;
    Ok(hex::encode(hasher.finalize()))
}

pub fn write_manifest(evidence_dir: &Path, manifest: &Manifest) -> io::Result<PathBuf> {
    fs::create_dir_all(evidence_dir)?;
    let path = evidence_dir.join("manifest.json");
    let f = File::create(&path)?;
    serde_json::to_writer_pretty(f, manifest)?;
    Ok(path)
}

pub fn evidence_dir_for(repo_root: &Path, run_id: &str) -> PathBuf {
    repo_root.join("evidence").join(run_id)
}

/// Open `path` for append, creating parent dirs. Used for build-metrics.jsonl.
pub fn append_line(path: &Path, line: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{line}")
}
