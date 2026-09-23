//! swf-cli robot-demo: run scenarios, the coverage matrix, and the protected
//! verifier against evidence.
//!
//! Assumes it is invoked from the repository root (the scripts in
//! scripts/robot-demo cd there). Override with ROBOT_DEMO_ROOT.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
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
        #[arg(long)]
        scenario: String,
        /// Simulator backend: mock | robosuite
        #[arg(long, default_value = "mock")]
        backend: String,
        /// Run identifier; evidence lands in evidence/<run-id>/
        #[arg(long)]
        run_id: Option<String>,
        /// Wall-clock timeout per bridge message, milliseconds
        #[arg(long, default_value = "10000")]
        timeout_ms: u64,
    },
    /// Run the coverage matrix (placements x freshness + stop + timeout)
    Matrix {
        #[arg(long, default_value = "mock")]
        backend: String,
        #[arg(long)]
        run_id: Option<String>,
        #[arg(long, default_value = "8000")]
        timeout_ms: u64,
    },
    /// Run the protected acceptance verifier over a run's evidence
    Validate {
        #[arg(long)]
        run_id: String,
    },
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
        .run();
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
        policy_max_observation_age_ms:
            robot_safety_gate::DEFAULT_MAX_OBSERVATION_AGE_MS,
        scope: "Rust contract checks, bridge checks and simulated robot scenarios. \
                Hardware HIL and physical validation are not performed.",
    };
    let path = evidence::write_manifest(&evidence::evidence_dir_for(root, run_id), &manifest)?;
    println!("manifest: {}", path.display());
    Ok(())
}

fn run_matrix(root: &Path, backend: &str, run_id: &str, timeout_ms: u64) -> Result<()> {
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
            if r.outcome == "timeout" || r.outcome == "protocol_error" || r.outcome == "bridge_died" {
                failures += 1;
            }
        }
    }

    for extra in matrix["extra_scenarios"].as_array().context("extra_scenarios")? {
        let name = extra.as_str().context("extra scenario name")?;
        let path = root.join(format!("demo/robot-sim/config/scenarios/{name}.json"));
        let r = run_scenario(root, name, &path, backend, run_id, timeout_ms)?;
        if r.outcome == "protocol_error" || r.outcome == "bridge_died" {
            failures += 1;
        }
    }

    println!("matrix complete: {failures} infrastructure failure(s)");
    Ok(())
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
                let path = root.join(format!(
                    "demo/robot-sim/config/scenarios/{scenario}.json"
                ));
                run_scenario(&root, &scenario, &path, &backend, &run_id, timeout_ms)?;
                write_manifest(&root, &run_id, &backend)?;
                println!("run-id: {run_id}");
            }
            RobotDemoCommands::Matrix {
                backend,
                run_id,
                timeout_ms,
            } => {
                let run_id = run_id.unwrap_or_else(default_run_id);
                run_matrix(&root, &backend, &run_id, timeout_ms)?;
                write_manifest(&root, &run_id, &backend)?;
                println!("run-id: {run_id}");
            }
            RobotDemoCommands::Validate { run_id } => {
                let verifier = root.join("demo/robot-sim/acceptance/verify_run.py");
                let status = std::process::Command::new(python())
                    .arg(verifier)
                    .arg(evidence::evidence_dir_for(&root, &run_id))
                    .status()
                    .context("running protected verifier")?;
                return Ok(ExitCode::from(status.code().unwrap_or(1) as u8));
            }
        },
    }
    Ok(ExitCode::SUCCESS)
}
