//! swf-cli robot-demo: run scenarios, the coverage matrix, and the protected
//! verifier against evidence.
//!
//! Assumes it is invoked from the repository root (the scripts in
//! scripts/robot-demo cd there). Override with ROBOT_DEMO_ROOT.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;
use swf_app::evidence::{
    self, ExecutableIdentity, Manifest, ScenarioResult, SimulatorIdentity, SourceIdentity,
};
use swf_app::session::{Session, SessionConfig};

#[derive(Parser)]
#[command(name = "swf-cli", version, about = "Software factory robot-demo CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Robot demonstration commands
    RobotDemo {
        #[command(subcommand)]
        cmd: RobotDemoCommands,
    },
}

#[derive(Subcommand)]
enum RobotDemoCommands {
    /// Run one scenario in lock-step with the simulator bridge
    Run {
        /// Scenario name from demo/robot-sim/config/scenarios (without .json)
        #[arg(long, value_parser = parse_identifier)]
        scenario: String,
        /// Simulator backend: mock | robosuite
        #[arg(long, default_value = "mock", value_parser = ["mock", "robosuite"])]
        backend: String,
        /// Run identifier; evidence lands in evidence/<run-id>/
        #[arg(long, value_parser = parse_identifier)]
        run_id: Option<String>,
        /// Wall-clock timeout per bridge message, milliseconds
        #[arg(long, default_value = "10000", value_parser = clap::value_parser!(u64).range(1..))]
        timeout_ms: u64,
    },
    /// Run the coverage matrix (placements x freshness + stop + timeout)
    Matrix {
        #[arg(long, default_value = "mock", value_parser = ["mock", "robosuite"])]
        backend: String,
        #[arg(long, value_parser = parse_identifier)]
        run_id: Option<String>,
        #[arg(long, default_value = "8000", value_parser = clap::value_parser!(u64).range(1..))]
        timeout_ms: u64,
    },
    /// Run the protected acceptance verifier over a run's evidence
    Validate {
        #[arg(long, value_parser = parse_identifier)]
        run_id: String,
        /// Verify only these named scenarios; repeat to select several.
        /// Omit to require the complete protected coverage matrix.
        #[arg(long, value_parser = parse_identifier)]
        scenario: Vec<String>,
    },
    /// Verify and summarize a controlled native/IB/cache benchmark receipt
    BuildProof {
        /// JSON receipt produced by scripts/robot-demo/ib-benchmark.sh
        #[arg(long)]
        receipt: PathBuf,
        /// Minimum independent samples required for every mode
        #[arg(long, default_value = "5", value_parser = parse_positive_usize)]
        min_samples: usize,
    },
    /// Extract documented counters from an Incredibuild Build History response
    BuildHistory {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        caption: String,
        /// Print only the build number (for cache-statistics collection)
        #[arg(long)]
        build_number_only: bool,
    },
    /// Create one normalized benchmark sample, failing on missing telemetry
    BuildSample {
        #[arg(long, value_parser = ["native", "ib-cold", "ib-parent-warm"])]
        mode: String,
        #[arg(long)]
        repetition: usize,
        #[arg(long)]
        wall_ms: u64,
        #[arg(long)]
        caption: String,
        #[arg(long)]
        source_revision: String,
        #[arg(long, requires = "cache")]
        history: Option<PathBuf>,
        #[arg(long, requires = "history")]
        cache: Option<PathBuf>,
    },
    /// Assemble normalized JSONL samples into a Rust-verifiable receipt
    BuildReceipt {
        #[arg(long)]
        samples: PathBuf,
        #[arg(long, value_parser = parse_identifier)]
        run_id: String,
        #[arg(long)]
        candidate_revision: String,
        #[arg(long)]
        parent_revision: String,
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BuildProof {
    schema_version: u32,
    run_id: String,
    candidate_revision: String,
    parent_revision: String,
    cache_scope: String,
    cache_cleared_before_each_cold_sample: bool,
    cache_cleared_before_each_parent_seed: bool,
    samples: Vec<BuildSample>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BuildSample {
    mode: String,
    repetition: usize,
    wall_ms: u64,
    build_caption: String,
    source_revision: String,
    remote_tasks: Option<u64>,
    local_tasks: Option<u64>,
    remote_core_time_s: Option<f64>,
    cache_hits: Option<u64>,
    cache_misses: Option<u64>,
}

#[derive(Debug, Serialize)]
struct IbHistory {
    build_number: u64,
    remote_tasks: u64,
    local_tasks: u64,
    remote_core_time_s: f64,
}

fn normalized(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}

fn value_for<'a>(
    record: &'a serde_json::Map<String, serde_json::Value>,
    aliases: &[&str],
) -> Option<&'a serde_json::Value> {
    record.iter().find_map(|(key, value)| {
        let key = normalized(key);
        aliases
            .iter()
            .any(|alias| key == normalized(alias))
            .then_some(value)
    })
}

fn walk_records<'a>(
    value: &'a serde_json::Value,
    records: &mut Vec<&'a serde_json::Map<String, serde_json::Value>>,
) {
    match value {
        serde_json::Value::Object(object) => {
            records.push(object);
            for child in object.values() {
                walk_records(child, records);
            }
        }
        serde_json::Value::Array(array) => {
            for child in array {
                walk_records(child, records);
            }
        }
        _ => {}
    }
}

fn as_u64(value: Option<&serde_json::Value>, label: &str) -> Result<u64> {
    value
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
        })
        .with_context(|| format!("{label} is missing or not an unsigned integer"))
}

fn as_f64(value: Option<&serde_json::Value>, label: &str) -> Result<f64> {
    value
        .and_then(|value| {
            value
                .as_f64()
                .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
        })
        .filter(|value| *value >= 0.0)
        .with_context(|| format!("{label} is missing, negative, or not numeric"))
}

fn parse_ib_history(document: &serde_json::Value, caption: &str) -> Result<IbHistory> {
    let mut records = Vec::new();
    walk_records(document, &mut records);
    let matches: Vec<_> = records
        .into_iter()
        .filter(|record| {
            value_for(record, &["buildCaption", "caption", "buildName", "name"])
                .and_then(serde_json::Value::as_str)
                == Some(caption)
        })
        .collect();
    if matches.len() != 1 {
        bail!(
            "expected exactly one Build History record for {caption:?}, found {}",
            matches.len()
        );
    }
    let record = matches[0];
    let status = value_for(record, &["buildStatus", "status"])
        .and_then(serde_json::Value::as_str)
        .context("build status is missing")?
        .to_ascii_lowercase();
    if !["success", "successful", "completed"].contains(&status.as_str()) {
        bail!("build {caption:?} is not successful: {status}");
    }
    Ok(IbHistory {
        build_number: as_u64(
            value_for(record, &["buildNumber", "buildId", "id"]),
            "build number",
        )?,
        remote_tasks: as_u64(
            value_for(record, &["numberOfRemoteTasks", "remoteTasks"]),
            "remote tasks",
        )?,
        local_tasks: as_u64(
            value_for(record, &["numberOfLocalTasks", "localTasks"]),
            "local tasks",
        )?,
        remote_core_time_s: as_f64(
            value_for(record, &["remoteCoreTime", "remoteCoreTimeSeconds"]),
            "remote core time",
        )?,
    })
}

fn parse_cache_counters(text: &str) -> Result<(u64, u64)> {
    fn counter(text: &str, wanted: &str) -> Result<u64> {
        let mut values = Vec::new();
        for line in text.lines() {
            let Some((label, value)) = line.split_once([':', '=']) else {
                continue;
            };
            let label = normalized(label);
            let accepted = [
                wanted.to_string(),
                format!("cache{wanted}"),
                format!("buildcache{wanted}"),
                format!("total{wanted}"),
                format!("totalcache{wanted}"),
                format!("totalbuildcache{wanted}"),
            ];
            if accepted.contains(&label) {
                if let Ok(value) = value.trim().parse::<u64>() {
                    values.push(value);
                }
            }
        }
        values.sort_unstable();
        values.dedup();
        if values.len() != 1 {
            bail!("expected one unambiguous cache {wanted} counter, found {values:?}");
        }
        Ok(values[0])
    }
    Ok((counter(text, "hits")?, counter(text, "misses")?))
}

#[allow(clippy::too_many_arguments)]
fn make_build_sample(
    mode: String,
    repetition: usize,
    wall_ms: u64,
    caption: String,
    source_revision: String,
    history: Option<PathBuf>,
    cache: Option<PathBuf>,
) -> Result<BuildSample> {
    if mode == "native" {
        if history.is_some() || cache.is_some() {
            bail!("native samples must not contain IB telemetry");
        }
        return Ok(BuildSample {
            mode,
            repetition,
            wall_ms,
            build_caption: caption,
            source_revision,
            remote_tasks: None,
            local_tasks: None,
            remote_core_time_s: None,
            cache_hits: None,
            cache_misses: None,
        });
    }
    let history_path = history.context("IB sample requires --history")?;
    let cache_path = cache.context("IB sample requires --cache")?;
    let history = parse_ib_history(
        &serde_json::from_str(&std::fs::read_to_string(&history_path)?)
            .with_context(|| format!("parsing {}", history_path.display()))?,
        &caption,
    )?;
    let (cache_hits, cache_misses) = parse_cache_counters(&std::fs::read_to_string(&cache_path)?)?;
    Ok(BuildSample {
        mode,
        repetition,
        wall_ms,
        build_caption: caption,
        source_revision,
        remote_tasks: Some(history.remote_tasks),
        local_tasks: Some(history.local_tasks),
        remote_core_time_s: Some(history.remote_core_time_s),
        cache_hits: Some(cache_hits),
        cache_misses: Some(cache_misses),
    })
}

#[derive(Debug)]
struct ModeStats {
    median_ms: f64,
    min_ms: u64,
    max_ms: u64,
}

fn mode_stats(samples: &[&BuildSample]) -> ModeStats {
    let mut walls: Vec<u64> = samples.iter().map(|sample| sample.wall_ms).collect();
    walls.sort_unstable();
    let middle = walls.len() / 2;
    let median_ms = if walls.len().is_multiple_of(2) {
        (walls[middle - 1] as f64 + walls[middle] as f64) / 2.0
    } else {
        walls[middle] as f64
    };
    ModeStats {
        median_ms,
        min_ms: walls[0],
        max_ms: walls[walls.len() - 1],
    }
}

fn validate_build_proof(
    proof: &BuildProof,
    min_samples: usize,
) -> Result<BTreeMap<String, ModeStats>> {
    if proof.schema_version != 1 {
        bail!("unsupported build-proof schema {}", proof.schema_version);
    }
    parse_identifier(&proof.run_id).map_err(anyhow::Error::msg)?;
    if proof.candidate_revision.is_empty() || proof.parent_revision.is_empty() {
        bail!("candidate_revision and parent_revision are required");
    }
    if proof.cache_scope != "local-user" {
        bail!(
            "expected isolated local-user cache scope, got {}",
            proof.cache_scope
        );
    }
    if !proof.cache_cleared_before_each_cold_sample {
        bail!("cold cache was not cleared before every independent sample");
    }
    if !proof.cache_cleared_before_each_parent_seed {
        bail!("cache was not cleared before every parent-seed sample");
    }

    let mut by_mode: BTreeMap<String, Vec<&BuildSample>> = BTreeMap::new();
    for sample in &proof.samples {
        if !["native", "ib-cold", "ib-parent-warm"].contains(&sample.mode.as_str()) {
            bail!("unknown benchmark mode {}", sample.mode);
        }
        if sample.wall_ms == 0 || sample.build_caption.is_empty() {
            bail!(
                "sample {} / {} has no wall time or caption",
                sample.mode,
                sample.repetition
            );
        }
        if sample.source_revision != proof.candidate_revision {
            bail!(
                "sample {} / {} built {}, expected candidate {}",
                sample.mode,
                sample.repetition,
                sample.source_revision,
                proof.candidate_revision
            );
        }
        if sample.mode == "native" {
            if sample.remote_tasks.is_some()
                || sample.remote_core_time_s.is_some()
                || sample.cache_hits.is_some()
            {
                bail!(
                    "native sample {} contains IB-only telemetry",
                    sample.repetition
                );
            }
        } else {
            if sample.remote_tasks.unwrap_or(0) == 0 {
                bail!(
                    "{} sample {} has no verified remote tasks",
                    sample.mode,
                    sample.repetition
                );
            }
            if sample.remote_core_time_s.unwrap_or(0.0) <= 0.0 {
                bail!(
                    "{} sample {} has no verified remote core time",
                    sample.mode,
                    sample.repetition
                );
            }
            if sample.local_tasks.is_none() || sample.cache_misses.is_none() {
                bail!(
                    "{} sample {} has incomplete IB telemetry",
                    sample.mode,
                    sample.repetition
                );
            }
        }
        if sample.mode == "ib-cold" && sample.cache_hits != Some(0) {
            bail!(
                "ib-cold sample {} was not empty-cache (hits={:?})",
                sample.repetition,
                sample.cache_hits
            );
        }
        if sample.mode == "ib-parent-warm" && sample.cache_hits.unwrap_or(0) == 0 {
            bail!(
                "ib-parent-warm sample {} has no verified cache hits",
                sample.repetition
            );
        }
        by_mode.entry(sample.mode.clone()).or_default().push(sample);
    }

    let mut stats = BTreeMap::new();
    for mode in ["native", "ib-cold", "ib-parent-warm"] {
        let samples = by_mode
            .get(mode)
            .with_context(|| format!("missing benchmark mode {mode}"))?;
        if samples.len() < min_samples {
            bail!(
                "{mode} has {} sample(s), require at least {min_samples}",
                samples.len()
            );
        }
        let mut repetitions: Vec<usize> = samples.iter().map(|sample| sample.repetition).collect();
        repetitions.sort_unstable();
        repetitions.dedup();
        if repetitions.len() != samples.len() {
            bail!("{mode} contains duplicate repetition numbers");
        }
        stats.insert(mode.to_string(), mode_stats(samples));
    }
    Ok(stats)
}

fn print_build_proof(proof: &BuildProof, min_samples: usize) -> Result<()> {
    let stats = validate_build_proof(proof, min_samples)?;
    println!("BUILD PROOF PASS  run={}", proof.run_id);
    println!(
        "candidate={} parent={}",
        proof.candidate_revision, proof.parent_revision
    );
    for mode in ["native", "ib-cold", "ib-parent-warm"] {
        let s = &stats[mode];
        println!(
            "{mode:<15} median={:>8.1}ms range={:>6}..{:>6}ms",
            s.median_ms, s.min_ms, s.max_ms
        );
    }
    let native = stats["native"].median_ms;
    for mode in ["ib-cold", "ib-parent-warm"] {
        let measured = stats[mode].median_ms;
        println!(
            "{mode:<15} measured ratio={:.3}x vs native; saved={:.0}ms",
            native / measured,
            native - measured
        );
    }
    println!(
        "distribution verified for every IB sample; parent-warmed cache hits verified for every warm sample"
    );
    Ok(())
}

fn parse_identifier(value: &str) -> std::result::Result<String, String> {
    if value.len() > 128
        || !value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err(
            "use 1–128 ASCII letters, digits, '.', '_' or '-', starting with a letter or digit"
                .into(),
        );
    }
    Ok(value.to_owned())
}

fn parse_positive_usize(value: &str) -> std::result::Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|parsed| *parsed > 0)
        .ok_or_else(|| "must be a positive integer".into())
}

fn expects_timeout(path: &Path) -> Result<bool> {
    let scenario: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path)?)
        .with_context(|| format!("parsing {}", path.display()))?;
    Ok(scenario["expect"] == "timeout")
}

fn infrastructure_failure(result: &ScenarioResult, timeout_expected: bool) -> bool {
    matches!(
        result.outcome.as_str(),
        "protocol_error" | "protocol_violation" | "bridge_died"
    ) || (result.outcome == "timeout" && !timeout_expected)
}

fn repo_root() -> PathBuf {
    std::env::var("ROBOT_DEMO_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().expect("cwd"))
}

fn python() -> String {
    std::env::var("ROBOT_DEMO_PYTHON").unwrap_or_else(|_| "python3".into())
}

fn default_run_id() -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("run-{ms}")
}

#[allow(clippy::too_many_arguments)]
fn run_scenario(
    root: &Path,
    scenario_name: &str,
    scenario_path: &Path,
    backend: &str,
    run_id: &str,
    timeout_ms: u64,
) -> Result<ScenarioResult> {
    parse_identifier(run_id).map_err(anyhow::Error::msg)?;
    parse_identifier(scenario_name).map_err(anyhow::Error::msg)?;
    let cfg = SessionConfig {
        run_id: run_id.to_string(),
        scenario_name: scenario_name.to_string(),
        scenario_path: scenario_path.to_path_buf(),
        bridge_path: root.join("demo/robot-sim/bridge.py"),
        python: python(),
        backend: backend.to_string(),
        evidence_dir: evidence::evidence_dir_for(root, run_id),
        proposal_timeout: Duration::from_millis(timeout_ms),
        policy: robot_safety_gate::Policy::default(),
    };
    let result = Session::spawn(cfg)
        .with_context(|| format!("spawning bridge for scenario {scenario_name}"))?
        .run()
        .with_context(|| format!("recording evidence for scenario {scenario_name}"))?;
    println!(
        "SCENARIO {scenario_name:<28} outcome={:<16} success={:<5} dispatches={} decisions={} wall={}ms",
        result.outcome, result.success, result.task_dispatches, result.decisions, result.wall_time_ms
    );
    for r in &result.rejections {
        println!("  rejection: {r}");
    }
    evidence::append_scenario_result(&evidence::evidence_dir_for(root, run_id), &result)?;
    Ok(result)
}

fn write_manifest(root: &Path, run_id: &str, backend: &str) -> Result<()> {
    let exe = std::env::current_exe()?;
    let git = |args: &[&str]| -> String {
        std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".into())
    };
    let python_version = std::process::Command::new(python())
        .arg("-c")
        .arg("import platform;print(platform.python_version())")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into());

    let manifest = Manifest {
        run_id: run_id.to_string(),
        created_unix_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0),
        source: SourceIdentity {
            git_commit: git(&["rev-parse", "HEAD"]),
            git_dirty: !git(&["status", "--porcelain"]).is_empty(),
            patch_applied: std::env::var("ROBOT_DEMO_PATCH").ok(),
        },
        executable: ExecutableIdentity {
            sha256: evidence::sha256_file(&exe)?,
            path: exe,
        },
        simulator: SimulatorIdentity {
            backend: backend.to_string(),
            python: python(),
            python_version,
            platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        },
        policy_max_observation_age_ms: robot_safety_gate::DEFAULT_MAX_OBSERVATION_AGE_MS,
        scope: "Rust contract checks, bridge checks and simulated robot scenarios. \
                Hardware HIL and physical validation are not performed.",
    };
    let path = evidence::write_manifest(&evidence::evidence_dir_for(root, run_id), &manifest)?;
    println!("manifest: {}", path.display());
    Ok(())
}

fn run_matrix(root: &Path, backend: &str, run_id: &str, timeout_ms: u64) -> Result<u32> {
    let matrix_path = root.join("demo/robot-sim/config/coverage-matrix.json");
    let matrix: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&matrix_path)?)
            .with_context(|| format!("parsing {}", matrix_path.display()))?;

    let gen_dir = evidence::evidence_dir_for(root, run_id).join("generated-scenarios");
    std::fs::create_dir_all(&gen_dir)?;

    let placements = matrix["placements"].as_array().context("placements")?;
    let freshness = matrix["freshness_ms"].as_array().context("freshness_ms")?;

    let mut failures = 0_u32;
    for p in placements {
        for f in freshness {
            let fms = f.as_u64().context("freshness ms")?;
            let expect = if fms > robot_safety_gate::DEFAULT_MAX_OBSERVATION_AGE_MS {
                "rejected_stale"
            } else {
                "cube_lifted"
            };
            let name = format!("{}-f{fms}", p["id"].as_str().context("placement id")?);
            parse_identifier(&name).map_err(anyhow::Error::msg)?;
            let scenario = serde_json::json!({
                "name": name,
                "placement": {"x": p["x"], "y": p["y"]},
                "staleness_ms": fms,
                "stop_at_segment": null,
                "stall_before_proposal_ms": 0,
                "expect": expect,
            });
            let path = gen_dir.join(format!("{name}.json"));
            std::fs::write(&path, serde_json::to_string_pretty(&scenario)?)?;
            let r = run_scenario(root, &name, &path, backend, run_id, timeout_ms)?;
            if infrastructure_failure(&r, false) {
                failures += 1;
            }
        }
    }

    for extra in matrix["extra_scenarios"]
        .as_array()
        .context("extra_scenarios")?
    {
        let name = extra.as_str().context("extra scenario name")?;
        parse_identifier(name).map_err(anyhow::Error::msg)?;
        let path = root.join(format!("demo/robot-sim/config/scenarios/{name}.json"));
        let timeout_expected = expects_timeout(&path)?;
        let r = run_scenario(root, name, &path, backend, run_id, timeout_ms)?;
        if infrastructure_failure(&r, timeout_expected) {
            failures += 1;
        }
    }

    println!("matrix complete: {failures} infrastructure failure(s)");
    Ok(failures)
}

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();
    let root = repo_root();

    match cli.command {
        Commands::RobotDemo { cmd } => match cmd {
            RobotDemoCommands::Run {
                scenario,
                backend,
                run_id,
                timeout_ms,
            } => {
                let run_id = run_id.unwrap_or_else(default_run_id);
                let path = root.join(format!("demo/robot-sim/config/scenarios/{scenario}.json"));
                let timeout_expected = expects_timeout(&path)?;
                let result = run_scenario(&root, &scenario, &path, &backend, &run_id, timeout_ms)?;
                write_manifest(&root, &run_id, &backend)?;
                println!("run-id: {run_id}");
                if infrastructure_failure(&result, timeout_expected) {
                    return Ok(ExitCode::FAILURE);
                }
            }
            RobotDemoCommands::Matrix {
                backend,
                run_id,
                timeout_ms,
            } => {
                let run_id = run_id.unwrap_or_else(default_run_id);
                let failures = run_matrix(&root, &backend, &run_id, timeout_ms)?;
                write_manifest(&root, &run_id, &backend)?;
                println!("run-id: {run_id}");
                if failures > 0 {
                    return Ok(ExitCode::FAILURE);
                }
            }
            RobotDemoCommands::Validate { run_id, scenario } => {
                let verifier = root.join("demo/robot-sim/acceptance/verify_run.py");
                let mut command = std::process::Command::new(python());
                command
                    .arg(verifier)
                    .arg(evidence::evidence_dir_for(&root, &run_id));
                for name in scenario {
                    command.arg("--scenario").arg(name);
                }
                let status = command.status().context("running protected verifier")?;
                return Ok(ExitCode::from(status.code().unwrap_or(1) as u8));
            }
            RobotDemoCommands::BuildProof {
                receipt,
                min_samples,
            } => {
                let proof: BuildProof = serde_json::from_str(
                    &std::fs::read_to_string(&receipt)
                        .with_context(|| format!("reading {}", receipt.display()))?,
                )
                .with_context(|| format!("parsing {}", receipt.display()))?;
                print_build_proof(&proof, min_samples)?;
            }
            RobotDemoCommands::BuildHistory {
                input,
                caption,
                build_number_only,
            } => {
                let history = parse_ib_history(
                    &serde_json::from_str(&std::fs::read_to_string(&input)?)
                        .with_context(|| format!("parsing {}", input.display()))?,
                    &caption,
                )?;
                if build_number_only {
                    println!("{}", history.build_number);
                } else {
                    println!("{}", serde_json::to_string(&history)?);
                }
            }
            RobotDemoCommands::BuildSample {
                mode,
                repetition,
                wall_ms,
                caption,
                source_revision,
                history,
                cache,
            } => {
                let sample = make_build_sample(
                    mode,
                    repetition,
                    wall_ms,
                    caption,
                    source_revision,
                    history,
                    cache,
                )?;
                println!("{}", serde_json::to_string(&sample)?);
            }
            RobotDemoCommands::BuildReceipt {
                samples,
                run_id,
                candidate_revision,
                parent_revision,
                output,
            } => {
                let samples = std::fs::read_to_string(&samples)?
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(serde_json::from_str)
                    .collect::<std::result::Result<Vec<BuildSample>, _>>()?;
                let proof = BuildProof {
                    schema_version: 1,
                    run_id,
                    candidate_revision,
                    parent_revision,
                    cache_scope: "local-user".into(),
                    cache_cleared_before_each_cold_sample: true,
                    cache_cleared_before_each_parent_seed: true,
                    samples,
                };
                if let Some(parent) = output.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&output, serde_json::to_string_pretty(&proof)? + "\n")?;
                println!("{}", output.display());
            }
        },
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_cannot_escape_evidence_or_scenario_directories() {
        for value in [
            "",
            ".",
            "..",
            "../outside",
            "a/b",
            "a\\b",
            "/tmp/run",
            "with space",
        ] {
            assert!(parse_identifier(value).is_err(), "{value:?}");
        }
        for value in ["fresh_lift", "run-123", "v1.2-test"] {
            assert_eq!(parse_identifier(value).unwrap(), value);
        }
    }

    #[test]
    fn clap_rejects_invalid_backend_timeout_and_paths() {
        for option in [
            ["--backend", "unknown"],
            ["--timeout-ms", "0"],
            ["--run-id", "../outside"],
        ] {
            assert!(Cli::try_parse_from([
                "swf-cli",
                "robot-demo",
                "run",
                "--scenario",
                "fresh_lift",
                option[0],
                option[1],
            ])
            .is_err());
        }
    }

    #[test]
    fn explicit_validation_scenarios_are_repeatable() {
        let cli = Cli::try_parse_from([
            "swf-cli",
            "robot-demo",
            "validate",
            "--run-id",
            "run-1",
            "--scenario",
            "fresh_lift",
            "--scenario",
            "stale_600ms",
        ])
        .unwrap();
        let Commands::RobotDemo {
            cmd: RobotDemoCommands::Validate { scenario, .. },
        } = cli.command
        else {
            panic!("expected validate command");
        };
        assert_eq!(scenario, ["fresh_lift", "stale_600ms"]);
    }

    #[test]
    fn only_expected_timeouts_are_nonfatal_infrastructure_results() {
        let mut result = ScenarioResult {
            scenario: "test".into(),
            backend: "mock".into(),
            outcome: "timeout".into(),
            success: false,
            task_dispatches: 0,
            decisions: 0,
            rejections: vec![],
            ticks: 0,
            wall_time_ms: 0,
        };
        assert!(infrastructure_failure(&result, false));
        assert!(!infrastructure_failure(&result, true));
        for outcome in ["protocol_error", "protocol_violation", "bridge_died"] {
            result.outcome = outcome.into();
            assert!(infrastructure_failure(&result, true));
        }
        for outcome in ["cube_lifted", "rejected_stale", "emergency_stop"] {
            result.outcome = outcome.into();
            assert!(!infrastructure_failure(&result, false));
        }
    }

    fn proof_sample(
        mode: &str,
        repetition: usize,
        remote_tasks: Option<u64>,
        cache_hits: Option<u64>,
    ) -> BuildSample {
        BuildSample {
            mode: mode.into(),
            repetition,
            wall_ms: 1_000 + repetition as u64,
            build_caption: format!("proof-{mode}-{repetition}"),
            source_revision: "candidate".into(),
            remote_tasks,
            local_tasks: remote_tasks.map(|_| 2),
            remote_core_time_s: remote_tasks.map(|_| 0.25),
            cache_hits,
            cache_misses: remote_tasks.map(|_| 3),
        }
    }

    fn complete_build_proof() -> BuildProof {
        let mut samples = Vec::new();
        for repetition in 1..=5 {
            samples.push(proof_sample("native", repetition, None, None));
            samples.push(proof_sample("ib-cold", repetition, Some(4), Some(0)));
            samples.push(proof_sample("ib-parent-warm", repetition, Some(1), Some(3)));
        }
        BuildProof {
            schema_version: 1,
            run_id: "proof-1".into(),
            candidate_revision: "candidate".into(),
            parent_revision: "parent".into(),
            cache_scope: "local-user".into(),
            cache_cleared_before_each_cold_sample: true,
            cache_cleared_before_each_parent_seed: true,
            samples,
        }
    }

    #[test]
    fn build_proof_requires_distribution_and_cache_hits() {
        let mut proof = complete_build_proof();
        assert!(validate_build_proof(&proof, 5).is_ok());

        proof.cache_cleared_before_each_parent_seed = false;
        assert!(validate_build_proof(&proof, 5)
            .unwrap_err()
            .to_string()
            .contains("parent-seed"));
        proof.cache_cleared_before_each_parent_seed = true;

        proof.samples[1].remote_tasks = Some(0);
        assert!(validate_build_proof(&proof, 5)
            .unwrap_err()
            .to_string()
            .contains("no verified remote tasks"));
        proof.samples[1].remote_tasks = Some(4);

        let warm = proof
            .samples
            .iter_mut()
            .find(|sample| sample.mode == "ib-parent-warm")
            .unwrap();
        warm.cache_hits = Some(0);
        assert!(validate_build_proof(&proof, 5)
            .unwrap_err()
            .to_string()
            .contains("no verified cache hits"));
    }

    #[test]
    fn build_proof_requires_five_independent_samples_per_mode() {
        let mut proof = complete_build_proof();
        proof
            .samples
            .retain(|sample| sample.mode != "native" || sample.repetition != 5);
        assert!(validate_build_proof(&proof, 5)
            .unwrap_err()
            .to_string()
            .contains("require at least 5"));
    }

    #[test]
    fn ib_history_parser_selects_caption_and_documented_counters() {
        let history = serde_json::json!({
            "builds": [
                {
                    "buildCaption": "other",
                    "buildNumber": 40,
                    "buildStatus": "Success",
                    "numberOfRemoteTasks": 1,
                    "numberOfLocalTasks": 1,
                    "remoteCoreTime": 0.1
                },
                {
                    "buildCaption": "rcc-proof-ib-cold-1",
                    "buildNumber": 41,
                    "buildStatus": "Success",
                    "numberOfRemoteTasks": 7,
                    "numberOfLocalTasks": 2,
                    "remoteCoreTime": 1.25
                }
            ]
        });
        let parsed = parse_ib_history(&history, "rcc-proof-ib-cold-1").unwrap();
        assert_eq!(parsed.build_number, 41);
        assert_eq!(parsed.remote_tasks, 7);
        assert_eq!(parsed.local_tasks, 2);
        assert_eq!(parsed.remote_core_time_s, 1.25);
        assert!(parse_ib_history(&history, "missing").is_err());
    }

    #[test]
    fn cache_parser_requires_unambiguous_hits_and_misses() {
        assert_eq!(
            parse_cache_counters("Build Cache Hits: 9\nBuild Cache Misses: 3\n").unwrap(),
            (9, 3)
        );
        assert!(parse_cache_counters("Build Cache Misses: 3\n").is_err());
        assert!(parse_cache_counters(
            "Build Cache Hits: 9\nCache Hits: 8\nBuild Cache Misses: 3\n"
        )
        .is_err());
    }
}
