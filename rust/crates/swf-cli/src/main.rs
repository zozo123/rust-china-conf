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
        /// Wall clock at which the timed build started, ms since the epoch.
        /// Recorded by the runner; used to prove a cache clear preceded it.
        #[arg(long)]
        started_at_ms: u64,
        /// Transcript of a cache-clearing operation the runner actually
        /// performed (see scripts/robot-demo/cache-clear.sh). Repeat for
        /// several. Required for every Incredibuild mode: swf-cli never
        /// asserts a cache state that it did not observe.
        #[arg(long = "cache-clear")]
        cache_clear: Vec<PathBuf>,
        /// Build History response covering this sample's own build
        #[arg(long, requires = "cache")]
        history: Option<PathBuf>,
        /// Cache-statistics output for this sample's own build
        #[arg(long, requires = "history")]
        cache: Option<PathBuf>,
        /// Build History response for the parent-revision seed build that
        /// warmed the cache. Required for ib-parent-warm.
        #[arg(long, requires_all = ["parent_seed_cache", "parent_seed_caption", "parent_seed_revision", "parent_seed_started_at_ms"])]
        parent_seed_history: Option<PathBuf>,
        /// Cache-statistics output for the parent-revision seed build
        #[arg(long, requires = "parent_seed_history")]
        parent_seed_cache: Option<PathBuf>,
        /// Unique build caption of the parent-revision seed build
        #[arg(long, requires = "parent_seed_history")]
        parent_seed_caption: Option<String>,
        /// Source revision the seed build compiled
        #[arg(long, requires = "parent_seed_history")]
        parent_seed_revision: Option<String>,
        /// Wall clock at which the seed build started, ms since the epoch
        #[arg(long, requires = "parent_seed_history")]
        parent_seed_started_at_ms: Option<u64>,
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

/// On-disk build-proof schema. Version 1 carried three top-level booleans that
/// asserted the cache scope and the cache-clearing procedure. Nothing produced
/// them but the assembling process itself, so the validator that "checked" them
/// was checking its own literals. Version 2 removes them: cache state is now
/// carried per sample, transcribed from operations the runner actually
/// performed, and a sample that carries no such transcript does not validate.
const BUILD_PROOF_SCHEMA_VERSION: u32 = 2;

/// Header line every cache-clear transcript must start with.
const CACHE_CLEAR_MARKER: &str = "# swf-cache-clear v1";

/// The cache namespace a clearing operation acted on, as reported by the
/// Incredibuild cache-management tool. There is deliberately no `Default` and
/// no inference: a transcript whose argv swf-cli does not recognize yields
/// `Unknown`, and `Unknown` never validates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum CacheScope {
    LocalUser,
    Shared,
    Unknown,
}

impl std::fmt::Display for CacheScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            CacheScope::LocalUser => "local-user",
            CacheScope::Shared => "shared",
            CacheScope::Unknown => "unknown",
        })
    }
}

/// One cache-clearing operation the runner performed, transcribed from the
/// cache tool's own output and exit status. swf-cli only ever parses these; it
/// has no code path that constructs one.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CacheClear {
    scope: CacheScope,
    argv: String,
    exit_code: i32,
    started_at_ms: u64,
    completed_at_ms: u64,
    transcript_path: String,
    transcript_sha256: String,
}

/// The parent-revision build that warmed the cache for an `ib-parent-warm`
/// sample. Its counters come from the same Build History and cache-statistics
/// tools as a measured sample's, so "the cache was warmed by the parent" is an
/// observation rather than an assertion.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ParentSeed {
    build_caption: String,
    source_revision: String,
    started_at_ms: u64,
    remote_tasks: u64,
    cache_hits: u64,
    cache_misses: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BuildProof {
    schema_version: u32,
    run_id: String,
    candidate_revision: String,
    parent_revision: String,
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
    /// Wall clock at which the timed build started, ms since the epoch.
    started_at_ms: u64,
    /// Cache clears the runner performed for this sample, in the order it
    /// performed them. Empty means "nothing was observed", not "nothing
    /// happened"; the validator rejects an empty list for IB modes.
    #[serde(default)]
    cache_clears: Vec<CacheClear>,
    /// Present only for ib-parent-warm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parent_seed: Option<ParentSeed>,
    remote_tasks: Option<u64>,
    local_tasks: Option<u64>,
    remote_core_time_s: Option<f64>,
    cache_hits: Option<u64>,
    cache_misses: Option<u64>,
}

/// Everything `build-sample` needs for one sample. Grouped so that adding an
/// evidence source does not grow a positional argument list.
struct BuildSampleRequest {
    mode: String,
    repetition: usize,
    wall_ms: u64,
    caption: String,
    source_revision: String,
    started_at_ms: u64,
    cache_clear: Vec<PathBuf>,
    history: Option<PathBuf>,
    cache: Option<PathBuf>,
    parent_seed: Option<ParentSeedRequest>,
}

struct ParentSeedRequest {
    caption: String,
    revision: String,
    started_at_ms: u64,
    history: PathBuf,
    cache: PathBuf,
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

/// Read one `key=value` header out of a cache-clear transcript, refusing a
/// missing or repeated key rather than taking the first or the last.
fn clear_header<'a>(headers: &'a BTreeMap<String, Vec<&'a str>>, key: &str) -> Result<&'a str> {
    match headers.get(key).map(Vec::as_slice) {
        Some([only]) => Ok(only),
        Some(many) => bail!("cache-clear transcript repeats {key} {} times", many.len()),
        None => bail!("cache-clear transcript has no {key}"),
    }
}

/// Parse a transcript emitted by scripts/robot-demo/cache-clear.sh.
///
/// This is the only way a `CacheClear` can come into existence. Every field is
/// read out of the transcript; nothing is defaulted and nothing is inferred. A
/// scope swf-cli does not recognize becomes `CacheScope::Unknown`, which the
/// validator rejects, so an unrecognized cache tool fails the proof instead of
/// silently passing it.
fn parse_cache_clear(text: &str, path: &Path, sha256: String) -> Result<CacheClear> {
    let (header_block, _) = text
        .split_once("\n--- transcript ---")
        .context("cache-clear transcript has no '--- transcript ---' separator")?;
    let mut lines = header_block.lines();
    if lines.next().map(str::trim_end) != Some(CACHE_CLEAR_MARKER) {
        bail!("cache-clear transcript does not start with {CACHE_CLEAR_MARKER:?}");
    }
    let mut headers: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .with_context(|| format!("cache-clear transcript has a non-header line {line:?}"))?;
        headers
            .entry(key.trim().to_string())
            .or_default()
            .push(value.trim());
    }

    let argv = clear_header(&headers, "argv")?.to_string();
    let exit_code: i32 = clear_header(&headers, "exit_code")?
        .parse()
        .context("cache-clear exit_code is not an integer")?;
    if exit_code != 0 {
        bail!(
            "cache clear {argv:?} exited {exit_code}; the cache state after it is unknown \
             and swf-cli will not record it as cleared"
        );
    }
    let started_at_ms: u64 = clear_header(&headers, "started_at_ms")?
        .parse()
        .context("cache-clear started_at_ms is not an unsigned integer")?;
    let completed_at_ms: u64 = clear_header(&headers, "completed_at_ms")?
        .parse()
        .context("cache-clear completed_at_ms is not an unsigned integer")?;
    if completed_at_ms < started_at_ms || started_at_ms == 0 {
        bail!("cache clear {argv:?} has a nonsensical time range");
    }

    // Derived, never asserted: the scope follows from the arguments the runner
    // actually passed to the cache tool.
    let scope = match argv.split_whitespace().next() {
        Some("user") => CacheScope::LocalUser,
        Some("shared") | Some("service") | Some("all") | Some("global") => CacheScope::Shared,
        _ => CacheScope::Unknown,
    };

    Ok(CacheClear {
        scope,
        argv,
        exit_code,
        started_at_ms,
        completed_at_ms,
        transcript_path: path.display().to_string(),
        transcript_sha256: sha256,
    })
}

fn load_cache_clear(path: &Path) -> Result<CacheClear> {
    let sha256 = evidence::sha256_file(path)
        .with_context(|| format!("hashing cache-clear transcript {}", path.display()))?;
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading cache-clear transcript {}", path.display()))?;
    parse_cache_clear(&text, path, sha256)
        .with_context(|| format!("in cache-clear transcript {}", path.display()))
}

fn load_parent_seed(request: &ParentSeedRequest) -> Result<ParentSeed> {
    let history = parse_ib_history(
        &serde_json::from_str(&std::fs::read_to_string(&request.history)?)
            .with_context(|| format!("parsing {}", request.history.display()))?,
        &request.caption,
    )?;
    let (cache_hits, cache_misses) =
        parse_cache_counters(&std::fs::read_to_string(&request.cache)?)?;
    if request.started_at_ms == 0 {
        bail!("parent seed {:?} has no start time", request.caption);
    }
    Ok(ParentSeed {
        build_caption: request.caption.clone(),
        source_revision: request.revision.clone(),
        started_at_ms: request.started_at_ms,
        remote_tasks: history.remote_tasks,
        cache_hits,
        cache_misses,
    })
}

fn make_build_sample(request: BuildSampleRequest) -> Result<BuildSample> {
    let BuildSampleRequest {
        mode,
        repetition,
        wall_ms,
        caption,
        source_revision,
        started_at_ms,
        cache_clear,
        history,
        cache,
        parent_seed,
    } = request;

    if started_at_ms == 0 {
        bail!("--started-at-ms is required and must be a real wall clock reading");
    }

    if mode == "native" {
        if history.is_some() || cache.is_some() || parent_seed.is_some() {
            bail!("native samples must not contain IB telemetry");
        }
        if !cache_clear.is_empty() {
            bail!("native samples do not use the Incredibuild cache and must not claim a clear");
        }
        return Ok(BuildSample {
            mode,
            repetition,
            wall_ms,
            build_caption: caption,
            source_revision,
            started_at_ms,
            cache_clears: Vec::new(),
            parent_seed: None,
            remote_tasks: None,
            local_tasks: None,
            remote_core_time_s: None,
            cache_hits: None,
            cache_misses: None,
        });
    }

    // An IB sample without an observed cache operation is exactly the case the
    // old schema papered over with a literal `true`. Refuse to emit it.
    if cache_clear.is_empty() {
        bail!(
            "{mode} sample {repetition} requires at least one --cache-clear transcript; \
             swf-cli will not record a cache state it did not observe"
        );
    }
    if mode == "ib-parent-warm" && parent_seed.is_none() {
        bail!("ib-parent-warm sample {repetition} requires the --parent-seed-* evidence");
    }
    if mode == "ib-cold" && parent_seed.is_some() {
        bail!("ib-cold sample {repetition} must not carry a parent seed");
    }

    let cache_clears = cache_clear
        .iter()
        .map(|path| load_cache_clear(path))
        .collect::<Result<Vec<_>>>()?;
    let parent_seed = parent_seed.as_ref().map(load_parent_seed).transpose()?;

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
        started_at_ms,
        cache_clears,
        parent_seed,
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

/// A cache-using build in this receipt, used to prove that nothing ran between
/// a cache clear and the build it is claimed to have prepared.
#[derive(Debug)]
struct CacheBuild {
    started_at_ms: u64,
    label: String,
}

/// What the receipt's own records support about cache handling, derived rather
/// than asserted. `print_build_proof` reports these instead of echoing a flag.
#[derive(Debug)]
struct CacheEvidence {
    scope: CacheScope,
    clears: usize,
    warm_seeds: usize,
}

#[derive(Debug)]
struct ProofSummary {
    stats: BTreeMap<String, ModeStats>,
    cache: CacheEvidence,
}

/// The latest clear that finished at or before `before_ms`.
fn effective_clear(clears: &[CacheClear], before_ms: u64) -> Option<&CacheClear> {
    clears
        .iter()
        .filter(|clear| clear.completed_at_ms <= before_ms)
        .max_by_key(|clear| clear.completed_at_ms)
}

/// Any cache-using build that started strictly between the two instants. One of
/// these means the clear cannot be attributed to the build that follows it.
fn intervening_build(builds: &[CacheBuild], after_ms: u64, before_ms: u64) -> Option<&CacheBuild> {
    builds
        .iter()
        .find(|build| build.started_at_ms > after_ms && build.started_at_ms < before_ms)
}

fn check_clear_usable(clear: &CacheClear, what: &str) -> Result<()> {
    if clear.exit_code != 0 {
        bail!(
            "{what}: cache clear {:?} exited {}",
            clear.argv,
            clear.exit_code
        );
    }
    match clear.scope {
        CacheScope::LocalUser => {}
        CacheScope::Shared => bail!(
            "{what}: cache clear {:?} acted on a shared cache; this benchmark may only \
             clear the invoking user's own cache",
            clear.argv
        ),
        CacheScope::Unknown => bail!(
            "{what}: cache clear {:?} has an unrecognized scope, so the cache namespace \
             it emptied is unknown",
            clear.argv
        ),
    }
    if clear.transcript_sha256.len() != 64
        || !clear
            .transcript_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        bail!(
            "{what}: cache clear {:?} has no retained transcript digest",
            clear.argv
        );
    }
    if clear.transcript_path.is_empty() {
        bail!(
            "{what}: cache clear {:?} has no retained transcript",
            clear.argv
        );
    }
    Ok(())
}

/// Verify, from records only, that every Incredibuild sample ran against the
/// cache state it claims. Nothing here reads a field that swf-cli wrote from a
/// literal: the clears are transcribed from the cache tool, the seed counters
/// come from Incredibuild's own statistics, and the ordering is checked against
/// timestamps the runner took around each build.
fn verify_cache_chain(proof: &BuildProof) -> Result<CacheEvidence> {
    let mut builds: Vec<CacheBuild> = Vec::new();
    for sample in &proof.samples {
        if sample.mode == "native" {
            continue;
        }
        builds.push(CacheBuild {
            started_at_ms: sample.started_at_ms,
            label: format!("{} sample {}", sample.mode, sample.repetition),
        });
        if let Some(seed) = &sample.parent_seed {
            builds.push(CacheBuild {
                started_at_ms: seed.started_at_ms,
                label: format!("parent seed {}", seed.build_caption),
            });
        }
    }
    let mut seen: BTreeMap<u64, &str> = BTreeMap::new();
    for build in &builds {
        if let Some(other) = seen.insert(build.started_at_ms, &build.label) {
            bail!(
                "{} and {} report the same start time {}ms; cache ordering cannot be \
                 established",
                other,
                build.label,
                build.started_at_ms
            );
        }
    }

    let mut scope: Option<CacheScope> = None;
    let mut clears = 0usize;
    let mut warm_seeds = 0usize;

    for sample in &proof.samples {
        if sample.mode == "native" {
            continue;
        }
        let what = format!("{} sample {}", sample.mode, sample.repetition);
        if sample.cache_clears.is_empty() {
            bail!("{what} records no cache clear, so the cache it built against is unknown");
        }
        for clear in &sample.cache_clears {
            clears += 1;
            match scope {
                None => scope = Some(clear.scope),
                Some(previous) if previous != clear.scope => bail!(
                    "{what}: cache clears disagree about scope ({previous} vs {})",
                    clear.scope
                ),
                Some(_) => {}
            }
        }

        // The instant the cache had to be empty: for a cold sample that is its
        // own build; for a warm sample it is the parent seed that warms it.
        let (target_ms, target_label) = match (&sample.mode[..], &sample.parent_seed) {
            ("ib-cold", _) => (sample.started_at_ms, what.clone()),
            ("ib-parent-warm", Some(seed)) => {
                if seed.source_revision != proof.parent_revision {
                    bail!(
                        "{what}: parent seed built {}, expected parent {}",
                        seed.source_revision,
                        proof.parent_revision
                    );
                }
                if seed.started_at_ms >= sample.started_at_ms {
                    bail!("{what}: parent seed did not start before the measured build");
                }
                if seed.remote_tasks == 0 {
                    bail!("{what}: parent seed has no verified remote tasks");
                }
                if seed.cache_hits != 0 {
                    bail!(
                        "{what}: parent seed reported {} cache hit(s), so it did not run \
                         against an emptied cache",
                        seed.cache_hits
                    );
                }
                if seed.cache_misses == 0 {
                    bail!("{what}: parent seed populated no cache entries");
                }
                if let Some(between) =
                    intervening_build(&builds, seed.started_at_ms, sample.started_at_ms)
                {
                    bail!(
                        "{what}: {} ran between the parent seed and the measured build, so \
                         the warm cache cannot be attributed to the parent",
                        between.label
                    );
                }
                warm_seeds += 1;
                (seed.started_at_ms, format!("parent seed for {what}"))
            }
            ("ib-parent-warm", None) => bail!(
                "{what} records no parent seed, so its cache hits cannot be attributed to \
                 the parent revision"
            ),
            (other, _) => bail!("unknown benchmark mode {other}"),
        };

        let clear = effective_clear(&sample.cache_clears, target_ms).with_context(|| {
            format!("{target_label}: every recorded cache clear finished after the build started")
        })?;
        check_clear_usable(clear, &target_label)?;
        if let Some(between) = intervening_build(&builds, clear.completed_at_ms, target_ms) {
            bail!(
                "{target_label}: {} ran between the cache clear and this build, so the clear \
                 cannot be attributed to it",
                between.label
            );
        }
    }

    let scope = scope.unwrap_or(CacheScope::Unknown);
    if scope != CacheScope::LocalUser {
        bail!("expected isolated local-user cache scope, observed {scope}");
    }
    Ok(CacheEvidence {
        scope,
        clears,
        warm_seeds,
    })
}

fn validate_build_proof(proof: &BuildProof, min_samples: usize) -> Result<ProofSummary> {
    if proof.schema_version != BUILD_PROOF_SCHEMA_VERSION {
        if proof.schema_version == 1 {
            bail!(
                "build-proof schema 1 asserted its cache scope and cache-clearing procedure as \
                 literals that nothing observed; re-run scripts/robot-demo/ib-benchmark.sh to \
                 produce a schema {BUILD_PROOF_SCHEMA_VERSION} receipt"
            );
        }
        bail!("unsupported build-proof schema {}", proof.schema_version);
    }
    parse_identifier(&proof.run_id).map_err(anyhow::Error::msg)?;
    if proof.candidate_revision.is_empty() || proof.parent_revision.is_empty() {
        bail!("candidate_revision and parent_revision are required");
    }
    if proof.candidate_revision == proof.parent_revision {
        bail!("candidate_revision and parent_revision must differ");
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
        if sample.started_at_ms == 0 {
            bail!(
                "sample {} / {} has no recorded start time",
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
                || sample.parent_seed.is_some()
                || !sample.cache_clears.is_empty()
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
    let cache = verify_cache_chain(proof)?;
    Ok(ProofSummary { stats, cache })
}

fn print_build_proof(proof: &BuildProof, min_samples: usize) -> Result<()> {
    let ProofSummary { stats, cache } = validate_build_proof(proof, min_samples)?;
    println!("BUILD RECEIPT CONSISTENT  run={}", proof.run_id);
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
        "cache scope observed in {} clear transcript(s): {}; {} parent-seed build(s) attributed",
        cache.clears, cache.scope, cache.warm_seeds
    );
    println!(
        "CHECKED FROM RECORDS: >={min_samples} independent samples per mode with distinct \
         repetitions; one Build History record per caption reporting success; remote task and \
         remote core-time counters present for every IB sample; cold-cache hits==0 and \
         warm-cache hits>0 from the cache-statistics tool; every IB build preceded by a \
         transcribed local-user cache clear that exited 0, with no other cache-using build \
         between the clear and it."
    );
    println!(
        "NOT CHECKED: this is a consistency check over a receipt, not a proof. It cannot \
         detect a fabricated receipt, and it does not observe the cache itself. Re-verify the \
         retained transcripts and Build History responses independently."
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
                started_at_ms,
                cache_clear,
                history,
                cache,
                parent_seed_history,
                parent_seed_cache,
                parent_seed_caption,
                parent_seed_revision,
                parent_seed_started_at_ms,
            } => {
                // clap's `requires_all` guarantees these arrive together.
                let parent_seed = match (
                    parent_seed_history,
                    parent_seed_cache,
                    parent_seed_caption,
                    parent_seed_revision,
                    parent_seed_started_at_ms,
                ) {
                    (Some(history), Some(cache), Some(caption), Some(revision), Some(started)) => {
                        Some(ParentSeedRequest {
                            caption,
                            revision,
                            started_at_ms: started,
                            history,
                            cache,
                        })
                    }
                    _ => None,
                };
                let sample = make_build_sample(BuildSampleRequest {
                    mode,
                    repetition,
                    wall_ms,
                    caption,
                    source_revision,
                    started_at_ms,
                    cache_clear,
                    history,
                    cache,
                    parent_seed,
                })?;
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
                // This arm assembles; it does not attest. Every cache fact in
                // the receipt arrived inside a sample, parsed from evidence by
                // `build-sample`. Adding a cache field here would make the
                // validator check this function's opinion again -- see the
                // `cache_state_is_never_asserted_by_construction` guard.
                let proof = BuildProof {
                    schema_version: BUILD_PROOF_SCHEMA_VERSION,
                    run_id,
                    candidate_revision,
                    parent_revision,
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

    fn clear_at(completed_at_ms: u64) -> CacheClear {
        CacheClear {
            scope: CacheScope::LocalUser,
            argv: "user clear".into(),
            exit_code: 0,
            started_at_ms: completed_at_ms - 10,
            completed_at_ms,
            transcript_path: format!("evidence/raw/clear-{completed_at_ms}.txt"),
            transcript_sha256: "0123456789abcdef".repeat(4),
        }
    }

    /// A deterministic timeline, one second-scale block per repetition:
    /// native at +100, the cold clear at +200 and its build at +300, the warm
    /// clear at +400, its parent seed at +500 and the measured warm build at
    /// +600. Every cache-using build gets a distinct start.
    fn proof_sample(
        mode: &str,
        repetition: usize,
        remote_tasks: Option<u64>,
        cache_hits: Option<u64>,
    ) -> BuildSample {
        let base = repetition as u64 * 1_000_000;
        let (started_at_ms, cache_clears, parent_seed) = match mode {
            "native" => (base + 100, Vec::new(), None),
            "ib-cold" => (base + 300, vec![clear_at(base + 200)], None),
            _ => (
                base + 600,
                vec![clear_at(base + 400)],
                Some(ParentSeed {
                    build_caption: format!("proof-parent-seed-{repetition}"),
                    source_revision: "parent".into(),
                    started_at_ms: base + 500,
                    remote_tasks: 6,
                    cache_hits: 0,
                    cache_misses: 9,
                }),
            ),
        };
        BuildSample {
            mode: mode.into(),
            repetition,
            wall_ms: 1_000 + repetition as u64,
            build_caption: format!("proof-{mode}-{repetition}"),
            source_revision: "candidate".into(),
            started_at_ms,
            cache_clears,
            parent_seed,
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
            schema_version: BUILD_PROOF_SCHEMA_VERSION,
            run_id: "proof-1".into(),
            candidate_revision: "candidate".into(),
            parent_revision: "parent".into(),
            samples,
        }
    }

    fn rejection(proof: &BuildProof) -> String {
        validate_build_proof(proof, 5)
            .expect_err("expected this receipt to be rejected")
            .to_string()
    }

    fn warm_mut(proof: &mut BuildProof) -> &mut BuildSample {
        proof
            .samples
            .iter_mut()
            .find(|sample| sample.mode == "ib-parent-warm")
            .unwrap()
    }

    #[test]
    fn build_proof_requires_distribution_and_cache_hits() {
        let mut proof = complete_build_proof();
        assert!(validate_build_proof(&proof, 5).is_ok());

        proof.samples[1].remote_tasks = Some(0);
        assert!(rejection(&proof).contains("no verified remote tasks"));
        proof.samples[1].remote_tasks = Some(4);

        warm_mut(&mut proof).cache_hits = Some(0);
        assert!(rejection(&proof).contains("no verified cache hits"));
    }

    #[test]
    fn build_proof_requires_five_independent_samples_per_mode() {
        let mut proof = complete_build_proof();
        proof
            .samples
            .retain(|sample| sample.mode != "native" || sample.repetition != 5);
        assert!(rejection(&proof).contains("require at least 5"));
    }

    // --- the honesty regression suite -----------------------------------
    //
    // Schema 1 stamped the cache scope and the clearing procedure as literals
    // at receipt-assembly time and then validated those same literals, so the
    // three corresponding bails could never fire. Each test below drives one
    // of the replacements, which read only records the runner produced.

    #[test]
    fn schema_one_receipts_are_rejected_because_their_attestations_were_literals() {
        let mut proof = complete_build_proof();
        proof.schema_version = 1;
        let message = rejection(&proof);
        assert!(message.contains("schema 1"), "{message}");
        assert!(message.contains("literals"), "{message}");
    }

    #[test]
    fn an_ib_sample_without_an_observed_clear_is_unknown_not_clean() {
        let mut proof = complete_build_proof();
        proof.samples[1].cache_clears.clear();
        assert!(rejection(&proof).contains("records no cache clear"));

        let mut proof = complete_build_proof();
        warm_mut(&mut proof).cache_clears.clear();
        assert!(rejection(&proof).contains("records no cache clear"));
    }

    #[test]
    fn a_clear_is_rejected_unless_it_is_local_user_successful_and_retained() {
        let mut proof = complete_build_proof();
        proof.samples[1].cache_clears[0].scope = CacheScope::Shared;
        assert!(rejection(&proof).contains("shared cache"));

        let mut proof = complete_build_proof();
        proof.samples[1].cache_clears[0].scope = CacheScope::Unknown;
        assert!(rejection(&proof).contains("unrecognized scope"));

        let mut proof = complete_build_proof();
        proof.samples[1].cache_clears[0].exit_code = 3;
        assert!(rejection(&proof).contains("exited 3"));

        let mut proof = complete_build_proof();
        proof.samples[1].cache_clears[0].transcript_sha256 = "not-a-digest".into();
        assert!(rejection(&proof).contains("no retained transcript digest"));
    }

    #[test]
    fn a_clear_must_precede_the_build_it_is_claimed_to_have_prepared() {
        let mut proof = complete_build_proof();
        // The clear finished one millisecond after the timed build began.
        let start = proof.samples[1].started_at_ms;
        proof.samples[1].cache_clears[0].completed_at_ms = start + 1;
        assert!(rejection(&proof).contains("finished after the build started"));
    }

    #[test]
    fn a_clear_cannot_be_reused_across_an_intervening_cache_using_build() {
        let mut proof = complete_build_proof();
        // Repetition 2's cold clear is backdated ahead of repetition 1's warm
        // build, so another Incredibuild build ran in between and could have
        // repopulated the cache.
        let cold_two = proof
            .samples
            .iter_mut()
            .find(|sample| sample.mode == "ib-cold" && sample.repetition == 2)
            .unwrap();
        cold_two.cache_clears[0].completed_at_ms = 1_000_550;
        assert!(rejection(&proof).contains("ran between the cache clear"));
    }

    #[test]
    fn warm_cache_hits_must_be_attributable_to_a_recorded_parent_seed() {
        let mut proof = complete_build_proof();
        warm_mut(&mut proof).parent_seed = None;
        assert!(rejection(&proof).contains("records no parent seed"));

        let mut proof = complete_build_proof();
        warm_mut(&mut proof)
            .parent_seed
            .as_mut()
            .unwrap()
            .cache_hits = 2;
        assert!(rejection(&proof).contains("did not run against an emptied cache"));

        let mut proof = complete_build_proof();
        warm_mut(&mut proof)
            .parent_seed
            .as_mut()
            .unwrap()
            .source_revision = "candidate".into();
        assert!(rejection(&proof).contains("expected parent"));

        let mut proof = complete_build_proof();
        warm_mut(&mut proof)
            .parent_seed
            .as_mut()
            .unwrap()
            .remote_tasks = 0;
        assert!(rejection(&proof).contains("parent seed has no verified remote tasks"));
    }

    #[test]
    fn native_samples_may_not_carry_cache_evidence() {
        let mut proof = complete_build_proof();
        proof.samples[0].cache_clears.push(clear_at(50));
        assert!(rejection(&proof).contains("IB-only telemetry"));
    }

    #[test]
    fn cache_clear_transcripts_are_parsed_not_assumed() {
        let digest = "0123456789abcdef".repeat(4);
        let ok = "# swf-cache-clear v1\n\
                  argv=user clear\n\
                  exit_code=0\n\
                  started_at_ms=1000\n\
                  completed_at_ms=1200\n\
                  --- transcript ---\n\
                  Local user build cache cleared.\n";
        let parsed = parse_cache_clear(ok, Path::new("clear.txt"), digest.clone()).unwrap();
        assert_eq!(parsed.scope, CacheScope::LocalUser);
        assert_eq!(parsed.completed_at_ms, 1200);

        // A failed clear is never recorded as a clear.
        let failed = ok.replace("exit_code=0", "exit_code=1");
        assert!(
            parse_cache_clear(&failed, Path::new("clear.txt"), digest.clone())
                .unwrap_err()
                .to_string()
                .contains("unknown")
        );

        // An unfamiliar cache tool invocation yields Unknown, which the
        // validator rejects -- it is never optimistically read as local-user.
        let exotic = ok.replace("argv=user clear", "argv=whatever --purge");
        assert_eq!(
            parse_cache_clear(&exotic, Path::new("clear.txt"), digest.clone())
                .unwrap()
                .scope,
            CacheScope::Unknown
        );
        let shared = ok.replace("argv=user clear", "argv=shared clear");
        assert_eq!(
            parse_cache_clear(&shared, Path::new("clear.txt"), digest.clone())
                .unwrap()
                .scope,
            CacheScope::Shared
        );

        for broken in [
            ok.replace("# swf-cache-clear v1", "# something else"),
            ok.replace("exit_code=0\n", ""),
            ok.replace("argv=user clear", "argv=user clear\nargv=shared clear"),
            ok.replace("--- transcript ---", "transcript"),
        ] {
            assert!(parse_cache_clear(&broken, Path::new("clear.txt"), digest.clone()).is_err());
        }
    }

    /// The regression guard the fix exists for. If somebody restores the
    /// convenience of stamping cache state where the receipt is assembled,
    /// this fails -- even if every behavioural test above still passes,
    /// because a hardcoded `true` satisfies them all.
    #[test]
    fn cache_state_is_never_asserted_by_construction() {
        let source = include_str!("main.rs");

        // Assembled at runtime so this guard is not itself an occurrence.
        for banned in [
            format!("cache_cleared_before_each_{}", "cold_sample"),
            format!("cache_cleared_before_each_{}", "parent_seed"),
        ] {
            assert!(
                !source.contains(&banned),
                "{banned} is back: the validator would be checking its own literal again"
            );
        }

        let start = source
            .find("RobotDemoCommands::BuildReceipt {\n                samples,")
            .expect("build-receipt dispatch arm");
        let end = start
            + source[start..]
                .find("println!(\"{}\", output.display());")
                .expect("end of build-receipt dispatch arm");
        let arm = &source[start..end];
        for banned in ["CacheScope", "cache_clears", "parent_seed:", "cache_scope"] {
            assert!(
                !arm.contains(banned),
                "the build-receipt arm sets {banned} itself; cache state must reach the \
                 receipt only as evidence parsed by build-sample"
            );
        }
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
