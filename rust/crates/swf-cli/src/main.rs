//! swf-cli robot-demo: run scenarios, the coverage matrix, and the protected
//! verifier against evidence.
//!
//! Assumes it is invoked from the repository root (the scripts in
//! scripts/robot-demo cd there). Override with ROBOT_DEMO_ROOT.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
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
        /// Minimum samples required for every mode. They must carry distinct
        /// repetition numbers; that they were INDEPENDENT is not something
        /// this tool can establish, and it does not claim it.
        #[arg(long, default_value = "5", value_parser = parse_positive_usize)]
        min_samples: usize,
        /// Directory holding the retained cache-clear transcripts, for when a
        /// receipt is checked away from the machine that produced it: each
        /// transcript is then looked up by its recorded BASENAME inside this
        /// directory. Omitting it means the recorded paths must resolve as
        /// recorded. Neither form skips opening the file and recomputing its
        /// digest -- relocation moves the search, it does not waive it.
        #[arg(long)]
        transcripts: Option<PathBuf>,
        /// Build Cache hits an EMPTIED-cache build (ib-cold, and the parent
        /// seed) may report and still count as empty -- and, symmetrically,
        /// the number a warm build must EXCEED. Supplied by the verifier here,
        /// never by the receipt, and it tightens the warm test by as much as
        /// it loosens the cold one.
        ///
        /// The default 0 is the strict rule. Pass 1 for Rust: cargo invokes
        /// `rustc -vV` twice per build and the second invocation is served the
        /// entry the first stored, so an honest empty-cache Rust build reports
        /// one intra-build self-hit. Nothing here establishes that an allowed
        /// hit WAS that self-hit; the summary prints the floor and says so.
        #[arg(long, default_value = "0")]
        empty_cache_hit_floor: u64,
        /// What this check requires of DISTRIBUTION. Chosen by the verifier on
        /// the command line; the receipt cannot set it.
        ///
        /// `required` (the default) is the contract for a profile that
        /// declares rustc `allow_remote`: every IB sample, and every parent
        /// seed, must report at least one verified remote task and non-zero
        /// remote core time, or the run did not measure distribution at all.
        ///
        /// `excluded` is the contract for the CACHE-ONLY profile
        /// (rust/ib_profile.cache-only.xml: rustc `local_only` with
        /// `<ib_cache enabled="true"/>`, which keeps rustc intercepted and
        /// cached while never sending it to a helper). It is NOT a relaxation
        /// of `required`: it demands remote_tasks == 0 AND remote_core_time
        /// == 0 exactly, so a run that leaked even one task to a helper is
        /// REFUSED. Every cache check is unchanged -- the empty-cache floor
        /// still caps cold samples and warm samples must still exceed it --
        /// which is the point: with distribution verified ABSENT, the
        /// warm-vs-cold delta has nothing but the Build Cache to come from.
        #[arg(long, default_value = "required", value_parser = ["required", "excluded"])]
        distribution: String,
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

/// Accepted cache-clear transcript headers. v2 added `tool=` and `command=`
/// because a v1 transcript recorded only the post-shift `argv`, so it could not
/// say which tool had run — it could not evidence what it claimed to evidence.
const CACHE_CLEAR_MARKERS: [&str; 2] = ["# swf-cache-clear v1", "# swf-cache-clear v2"];

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

/// What the verifier requires of DISTRIBUTION, as opposed to caching.
///
/// Incredibuild's profile schema makes these independent knobs on one
/// declaration: `/opt/incredibuild/data/ib_profile.xsd` gives `type` the
/// enumeration {intercepted, static_intercepted, static_intercepted_fileops,
/// allow_remote, local_only} and makes `<ib_cache enabled="..."/>` a separate
/// child element of the same `<process>`. rustc can therefore be cached
/// without ever being distributed, and that configuration needs a contract of
/// its own.
///
/// Both variants are falsifiable demands, in opposite directions. There is
/// deliberately no variant meaning "do not look at the remote counters":
/// switching to `Excluded` to excuse a run with no remote tasks immediately
/// makes any remote task a rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Distribution {
    /// rustc declared `allow_remote`: work must actually have crossed the
    /// network. remote_tasks > 0 and remote_core_time > 0.
    Required,
    /// rustc declared `local_only`: nothing may have crossed the network, so
    /// whatever acceleration was measured is the Build Cache's alone.
    /// remote_tasks == 0 and remote_core_time == 0.
    Excluded,
}

impl Distribution {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "required" => Ok(Self::Required),
            "excluded" => Ok(Self::Excluded),
            other => bail!("unknown --distribution {other}; use required or excluded"),
        }
    }
}

impl std::fmt::Display for Distribution {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Required => "required",
            Self::Excluded => "excluded",
        })
    }
}

/// Where proof time looks for the transcripts a receipt names.
///
/// There is deliberately no variant meaning "do not look". A receipt is a
/// claim; the transcripts are the only thing in reach that can corroborate it;
/// a receipt whose transcripts cannot be opened is refused, never summarized.
/// `--transcripts` exists because a receipt is legitimately checked on a
/// machine other than the one that produced it, and `ib-benchmark.sh` records
/// host-absolute paths. It MOVES the search. It does not waive it.
#[derive(Debug, Clone)]
struct TranscriptSource {
    relocate: Option<PathBuf>,
}

impl TranscriptSource {
    /// Resolve every transcript at the path the receipt recorded.
    fn as_recorded() -> Self {
        Self { relocate: None }
    }

    /// Resolve every transcript by its recorded BASENAME inside `dir`.
    fn relocated(dir: PathBuf) -> Self {
        Self {
            relocate: Some(dir),
        }
    }

    fn describe(&self) -> String {
        match &self.relocate {
            None => "the paths recorded in the receipt".to_string(),
            Some(dir) => dir.display().to_string(),
        }
    }

    fn resolve(&self, recorded: &str) -> Result<PathBuf> {
        if recorded.trim().is_empty() {
            bail!("the receipt records no transcript path");
        }
        let recorded_path = Path::new(recorded);
        match &self.relocate {
            None => {
                if !recorded_path.is_file() {
                    bail!(
                        "transcript {recorded} is not present at its recorded path; if the \
                         retained transcripts were moved, or this receipt was produced on \
                         another host, pass --transcripts <dir>"
                    );
                }
                Ok(recorded_path.to_path_buf())
            }
            Some(dir) => {
                // Only the final component is honoured. `Path::file_name` never
                // yields "." or "..", so the join cannot leave `dir`:
                // relocation narrows the search and can never widen it into a
                // path the receipt chose. Without that, a receipt naming
                // "../somewhere/mine.txt" would pick its own evidence.
                let base = recorded_path.file_name().with_context(|| {
                    format!("transcript path {recorded:?} has no file name to relocate")
                })?;
                let candidate = dir.join(base);
                if !candidate.is_file() {
                    bail!(
                        "transcript {} (recorded as {recorded}) is not in {}; relocation moves \
                         the search, it does not waive it",
                        base.to_string_lossy(),
                        dir.display()
                    );
                }
                Ok(candidate)
            }
        }
    }
}

/// What a cache-clear transcript itself says, read out of the file before any
/// receipt is consulted. `CacheClear` is this plus the identity of the file it
/// came from (path and digest). Keeping them as two types is what makes "the
/// receipt disagrees with its own evidence" expressible at all: with one type
/// the comparison would be a field against itself, which is the schema-v1
/// tautology rebuilt one layer down.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ClearTranscript {
    /// 1 or 2: which entry of `CACHE_CLEAR_MARKERS` the file opened with.
    marker_version: u32,
    /// The tool the runner actually invoked. A v1 transcript recorded only the
    /// post-shift argv and so cannot answer this: `None`.
    tool: Option<String>,
    /// `tool` followed by `argv`, as the runner spelled it. v2 only.
    command: Option<String>,
    argv: String,
    scope: CacheScope,
    exit_code: i32,
    started_at_ms: u64,
    completed_at_ms: u64,
}

/// One transcript that was opened, re-hashed and re-parsed in this invocation.
/// Nothing in here was read from a receipt.
#[derive(Debug, Clone)]
struct Corroborated {
    digest: String,
    transcript: ClearTranscript,
}

/// Per-invocation record of what has actually been read off the filesystem, so
/// a transcript cited by several samples is hashed once and so the printed
/// counts describe files rather than JSON fields.
#[derive(Debug, Default)]
struct TranscriptLedger {
    /// Resolved path -> what that file actually says.
    read: BTreeMap<PathBuf, Corroborated>,
    /// Basename -> the single recorded path it may stand for, under relocation.
    basenames: BTreeMap<String, String>,
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

/// Resolve the initiator-local build number that the cache-statistics tool is
/// keyed by.
///
/// Incredibuild 4.30 reports `buildId` as an opaque string of the form
/// `uuid_<initiator-uuid>_buildid_<pid>_<NNNNNN>`, whose final zero-padded
/// segment is the initiator-local build number. That number -- not the opaque
/// string -- is what names
/// `/etc/incredibuild/db/incredibuildBuildReport_<n>.db`, and therefore what
/// `show_build_cache_statistics.sh <n>` accepts. A plain numeric field is still
/// preferred whenever the deployment provides one; the string is only
/// destructured as a fallback, and only when its final segment is entirely
/// digits. Anything else is an error rather than a guess.
fn parse_build_number(record: &serde_json::Map<String, serde_json::Value>) -> Result<u64> {
    if let Ok(number) = as_u64(
        value_for(record, &["buildNumber", "localBuildNumber"]),
        "build number",
    ) {
        return Ok(number);
    }
    if let Ok(number) = as_u64(value_for(record, &["buildId", "id"]), "build number") {
        return Ok(number);
    }
    let raw = value_for(record, &["buildId", "id"])
        .and_then(serde_json::Value::as_str)
        .context("build number is missing: no numeric buildNumber and no buildId string")?;
    let tail = raw
        .rsplit('_')
        .next()
        .filter(|segment| !segment.is_empty() && segment.chars().all(|c| c.is_ascii_digit()))
        .with_context(|| {
            format!("buildId {raw:?} does not end in a numeric build-number segment")
        })?;
    tail.parse::<u64>()
        .with_context(|| format!("buildId {raw:?} has an unparsable build-number segment {tail:?}"))
}

fn parse_ib_history(document: &serde_json::Value, caption: &str) -> Result<IbHistory> {
    let mut records = Vec::new();
    walk_records(document, &mut records);
    let matches: Vec<_> = records
        .into_iter()
        .filter(|record| {
            // buildTitle/title are Incredibuild 4.30's spelling. Without them
            // every record matches zero captions and the gate cannot read this
            // grid's Build History at all.
            value_for(
                record,
                &[
                    "buildCaption",
                    "caption",
                    "buildName",
                    "name",
                    "buildTitle",
                    "title",
                ],
            )
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
    // "succeeded" is Incredibuild 4.30's spelling of the same outcome.
    if !["success", "successful", "completed", "succeeded"].contains(&status.as_str()) {
        bail!("build {caption:?} is not successful: {status}");
    }
    Ok(IbHistory {
        build_number: parse_build_number(record)?,
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

/// Derived, never asserted: the cache namespace follows from the arguments the
/// runner actually passed to the cache tool. Lifted out of the parser so proof
/// time can re-derive it from the transcript instead of believing the receipt.
fn scope_of(argv: &str) -> CacheScope {
    match argv.split_whitespace().next() {
        Some("user") => CacheScope::LocalUser,
        Some("shared") | Some("service") | Some("all") | Some("global") => CacheScope::Shared,
        _ => CacheScope::Unknown,
    }
}

/// Parse a transcript emitted by scripts/robot-demo/cache-clear.sh into what
/// the FILE says, with no receipt in sight.
///
/// Every field is read out of the text; nothing is defaulted and nothing is
/// inferred. A scope swf-cli does not recognize becomes `CacheScope::Unknown`,
/// which the validator rejects, so an unrecognized cache tool fails the proof
/// instead of silently passing it.
///
/// A non-zero exit is REPORTED here rather than rejected, so that proof time
/// can say "the receipt claims 0, the transcript records 3" instead of losing
/// that disagreement inside a parse error. `parse_cache_clear` below keeps the
/// rejection, so the recording path is unchanged.
fn parse_clear_transcript(text: &str) -> Result<ClearTranscript> {
    let (header_block, _) = text
        .split_once("\n--- transcript ---")
        .context("cache-clear transcript has no '--- transcript ---' separator")?;
    let mut lines = header_block.lines();
    let header = lines.next().map(str::trim_end);
    let marker_version = CACHE_CLEAR_MARKERS
        .iter()
        .position(|marker| Some(*marker) == header)
        .map(|index| index as u32 + 1)
        .with_context(|| {
            format!(
                "cache-clear transcript does not start with one of {CACHE_CLEAR_MARKERS:?}; got {header:?}"
            )
        })?;
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
    let started_at_ms: u64 = clear_header(&headers, "started_at_ms")?
        .parse()
        .context("cache-clear started_at_ms is not an unsigned integer")?;
    let completed_at_ms: u64 = clear_header(&headers, "completed_at_ms")?
        .parse()
        .context("cache-clear completed_at_ms is not an unsigned integer")?;
    if completed_at_ms < started_at_ms || started_at_ms == 0 {
        bail!("cache clear {argv:?} has a nonsensical time range");
    }

    // v2 records which tool acted on the cache, because v1's `argv` was written
    // after the tool name had been shifted off and so could not say what ran. A
    // file carrying the v1 marker AND a tool= line could not have been written
    // by the v1 recorder, so it was edited afterwards and is not a transcript
    // of anything.
    let (tool, command) = if marker_version == 1 {
        for key in ["tool", "command"] {
            if headers.contains_key(key) {
                bail!(
                    "a v1 cache-clear transcript carries a {key} line that the v1 recorder \
                     could not have written"
                );
            }
        }
        (None, None)
    } else {
        let tool = clear_header(&headers, "tool")?.to_string();
        let command = clear_header(&headers, "command")?.to_string();
        if tool.is_empty() {
            bail!("cache-clear transcript names no tool");
        }
        let expected = format!("{tool} {argv}");
        if command.trim() != expected.trim() {
            bail!(
                "cache-clear transcript is internally inconsistent: command {command:?} is not \
                 tool {tool:?} followed by argv {argv:?}"
            );
        }
        (Some(tool), Some(command))
    };

    Ok(ClearTranscript {
        marker_version,
        tool,
        command,
        scope: scope_of(&argv),
        argv,
        exit_code,
        started_at_ms,
        completed_at_ms,
    })
}

/// The only way a `CacheClear` can come into existence: a transcript parsed out
/// of a file, plus the identity (path and digest) of the file it was parsed
/// from. swf-cli has no code path that constructs one from literals.
fn parse_cache_clear(text: &str, path: &Path, sha256: String) -> Result<CacheClear> {
    let parsed = parse_clear_transcript(text)?;
    if parsed.exit_code != 0 {
        bail!(
            "cache clear {:?} exited {}; the cache state after it is unknown \
             and swf-cli will not record it as cleared",
            parsed.argv,
            parsed.exit_code
        );
    }
    Ok(CacheClear {
        scope: parsed.scope,
        argv: parsed.argv,
        exit_code: parsed.exit_code,
        started_at_ms: parsed.started_at_ms,
        completed_at_ms: parsed.completed_at_ms,
        transcript_path: path.display().to_string(),
        transcript_sha256: sha256,
    })
}

/// Read a transcript ONCE and hash the bytes that were read.
///
/// `evidence::sha256_file` opens the file a second time, so the bytes it
/// hashes are not provably the bytes that are then parsed: a file can change
/// between the two opens, and a digest over content nobody parsed corroborates
/// nothing. Reading once and hashing the buffer removes the window entirely,
/// on the recording path and on the proof path alike.
///
/// This is byte-for-byte the digest `evidence::sha256_file` computes for the
/// same content -- `the_digest_is_computed_over_the_bytes_that_are_parsed`
/// pins that against a real file, so the two can never drift apart and a
/// receipt recorded by one remains checkable by the other.
fn read_and_digest(path: &Path) -> Result<(String, String)> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let digest = hex::encode(Sha256::digest(&bytes));
    let text = String::from_utf8(bytes)
        .with_context(|| format!("{} is not UTF-8 text", path.display()))?;
    Ok((digest, text))
}

fn load_cache_clear(path: &Path) -> Result<CacheClear> {
    let (sha256, text) = read_and_digest(path)
        .with_context(|| format!("hashing cache-clear transcript {}", path.display()))?;
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
    /// DISTINCT FILES opened, re-hashed and re-parsed in this invocation:
    /// `TranscriptLedger::read.len()`, taken after the walk rather than
    /// counted as the receipt is walked.
    ///
    /// The previous field incremented once per `CacheClear` JSON object, so
    /// it counted citations. A receipt chooses how many times it cites a
    /// transcript; it does not choose how many files exist. Citing one
    /// genuine transcript twenty times printed "20 corroborated clear
    /// transcript(s)" over as few as one file -- the printed number was back
    /// to being a restatement of the receipt, which is the defect this whole
    /// layer exists to kill.
    transcripts: usize,
    /// How many cache-clear records in the receipt cited those files. Printed
    /// BESIDE `transcripts`, never instead of it, so that a receipt citing
    /// one transcript twenty times is visible as exactly that.
    citations: usize,
    /// Parent seeds the receipt attributes its warm cache to. Receipt-asserted:
    /// a seed carries counters and a start time, and no document behind them is
    /// retained or re-read. Printed under its own label for that reason.
    warm_seeds: usize,
    /// Distinct cache tools named by those transcripts, read off disk. Printed
    /// so that a transcript recording `tool=/bin/true` is visible rather than
    /// parsed and discarded.
    tools: BTreeSet<String>,
    /// Distinct files among `transcripts` written by the v1 recorder, which
    /// shifted the tool name off before recording argv and so cannot say what
    /// ran. Ledger-derived, for the same reason as `transcripts`.
    anonymous_transcripts: usize,
    /// Where those transcripts were resolved from in this invocation.
    source: String,
}

/// What the VERIFIER asks of a receipt, as opposed to what the receipt says
/// about itself. Every field here is chosen on the command line by the person
/// doing the checking, and nothing in it can be set by the party that wrote
/// the receipt. That asymmetry is the whole point: a receipt may not choose
/// the standard it is judged by. There is deliberately no field here that
/// means "do not check".
#[derive(Debug)]
struct ProofPolicy {
    /// Samples required per mode, each with a distinct repetition number.
    min_samples: usize,
    /// Where the cache-clear transcripts are looked for. Moving the search is
    /// the only thing this can do; it can never waive it.
    transcripts: TranscriptSource,
    /// How many Build Cache hits a build on an EMPTIED cache may report and
    /// still count as empty-cache -- and, symmetrically, how many a warm build
    /// must EXCEED. It therefore tightens the warm test by exactly as much as
    /// it loosens the cold one, so it cannot be used as a pure loosener.
    ///
    /// The default is 0, which is the strict rule: an emptied cache served
    /// nothing. It exists because cargo invokes `rustc -vV` twice in one
    /// build and the second invocation is served the entry the first one
    /// stored, so a genuine empty-cache Rust build under Incredibuild reports
    /// exactly one intra-build self-hit (see `empty_cache_hit_floor` in
    /// scripts/robot-demo/ib-benchmark.sh). Without this the validator would
    /// refuse every honest Rust receipt, which is a false refusal, not rigour.
    ///
    /// NOTHING HERE ESTABLISHES that a hit allowed by this floor was that
    /// self-hit. The per-task Build Cache report the runner already writes
    /// does say so, and is not retained with a digest; until it is, a non-zero
    /// floor is the verifier's own judgement and `print_build_proof` prints it
    /// as such.
    empty_cache_hit_floor: u64,
    /// Whether this receipt is being judged as a DISTRIBUTION experiment or a
    /// CACHE-ONLY one. Each setting is a demand the other would reject, so
    /// this cannot be used to excuse a run: see `Distribution`.
    distribution: Distribution,
}

impl ProofPolicy {
    /// The strict standard: transcripts resolved as told, an emptied cache
    /// that served anything is not empty, and every IB build distributed.
    fn checking(min_samples: usize, transcripts: TranscriptSource) -> Self {
        Self {
            min_samples,
            transcripts,
            empty_cache_hit_floor: 0,
            distribution: Distribution::Required,
        }
    }

    fn with_empty_cache_hit_floor(mut self, floor: u64) -> Self {
        self.empty_cache_hit_floor = floor;
        self
    }

    fn with_distribution(mut self, distribution: Distribution) -> Self {
        self.distribution = distribution;
        self
    }
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

/// Open the transcript this clear names, recompute its digest, re-parse it, and
/// refuse unless the FILE corroborates every fact the RECEIPT asserts about it.
///
/// This is the function whose absence let a receipt citing
/// "/tmp/does-not-exist.txt" with a digest of sixty-four zeros print a measured
/// ratio and exit 0. `check_clear_usable` above asks whether the receipt's own
/// fields are well shaped, which is a question about a string; every answer it
/// can give is one the writer of the receipt chose. This asks whether there is
/// a file on disk that says the same thing.
///
/// Returns the path the transcript was actually read from -- so the caller
/// counts files rather than JSON objects, and can see one file cited twice --
/// and the cache tool the transcript names, or `None` for a v1 transcript,
/// whose recorder shifted the tool name off before writing `argv` and therefore
/// cannot say which tool acted on the cache. Cross-checking a v1 transcript's
/// argv against the receipt's argv is still worth doing -- it catches a
/// fabricated timeline -- but it cannot establish what ran, and the summary
/// says so rather than implying otherwise.
fn corroborate_clear(
    clear: &CacheClear,
    what: &str,
    source: &TranscriptSource,
    ledger: &mut TranscriptLedger,
) -> Result<(PathBuf, Option<String>)> {
    let subject = format!("{what}: cache clear {:?}", clear.argv);

    // Under relocation a basename stands for exactly one recorded path. Two
    // different recorded paths collapsing onto one file is an ambiguity about
    // which file this clear refers to, and an ambiguity is a refusal.
    if source.relocate.is_some() {
        let base = Path::new(&clear.transcript_path)
            .file_name()
            .map(|base| base.to_string_lossy().into_owned())
            .with_context(|| {
                format!(
                    "{subject}: transcript path {:?} has no file name",
                    clear.transcript_path
                )
            })?;
        match ledger.basenames.get(&base) {
            Some(first) if *first != clear.transcript_path => bail!(
                "{subject}: transcripts {:?} and {first:?} both relocate to {base:?}; which file \
                 this clear refers to cannot be established",
                clear.transcript_path
            ),
            _ => {
                ledger.basenames.insert(base, clear.transcript_path.clone());
            }
        }
    }

    let resolved = source
        .resolve(&clear.transcript_path)
        .with_context(|| subject.clone())?;

    let found = match ledger.read.get(&resolved) {
        Some(found) => found.clone(),
        None => {
            let (digest, text) = read_and_digest(&resolved)
                .with_context(|| format!("{subject}: opening transcript {}", resolved.display()))?;
            let transcript = parse_clear_transcript(&text).with_context(|| {
                format!("{subject}: re-parsing transcript {}", resolved.display())
            })?;
            let fresh = Corroborated { digest, transcript };
            ledger.read.insert(resolved.clone(), fresh.clone());
            fresh
        }
    };

    // The digest binds the receipt to THESE bytes -- the very bytes parsed
    // just above, hashed in the same read. A receipt that matches some other
    // file, or some other version of this file, is not evidence about the one
    // that was read.
    if found.digest != clear.transcript_sha256 {
        bail!(
            "{subject}: transcript {} hashes to {}, but the receipt records {}; this receipt \
             does not describe this file",
            resolved.display(),
            found.digest,
            clear.transcript_sha256
        );
    }

    // The receipt is a claim about this file; the file is the evidence. Where
    // they disagree the receipt is wrong. Every disagreement is collected, so
    // the operator sees the shape of the discrepancy and not only its first
    // field -- one fabricated transcript backing ten contradictory citations
    // looks very different from one mistyped timestamp.
    let actual = &found.transcript;
    let mut disagreements: Vec<String> = Vec::new();
    if actual.exit_code != clear.exit_code {
        disagreements.push(format!(
            "exit_code (receipt {}, transcript {})",
            clear.exit_code, actual.exit_code
        ));
    }
    if actual.started_at_ms != clear.started_at_ms {
        disagreements.push(format!(
            "started_at_ms (receipt {}, transcript {})",
            clear.started_at_ms, actual.started_at_ms
        ));
    }
    if actual.completed_at_ms != clear.completed_at_ms {
        disagreements.push(format!(
            "completed_at_ms (receipt {}, transcript {})",
            clear.completed_at_ms, actual.completed_at_ms
        ));
    }
    if actual.argv != clear.argv {
        disagreements.push(format!(
            "argv (receipt {:?}, transcript {:?})",
            clear.argv, actual.argv
        ));
    }
    if actual.scope != clear.scope {
        disagreements.push(format!(
            "scope (receipt {}, re-derived from the transcript's own arguments {})",
            clear.scope, actual.scope
        ));
    }
    if !disagreements.is_empty() {
        bail!(
            "{subject}: the receipt disagrees with transcript {}: {}",
            resolved.display(),
            disagreements.join("; ")
        );
    }

    // Redundant with check_clear_usable's reading of the receipt's own field,
    // but that field has now been shown equal to the file's, so this one is a
    // statement about the file.
    if actual.exit_code != 0 {
        bail!(
            "{subject}: transcript {} records exit {}",
            resolved.display(),
            actual.exit_code
        );
    }

    Ok((resolved, actual.tool.clone()))
}

/// Verify that every Incredibuild sample ran against the cache state it claims.
///
/// Nothing here reads a field that swf-cli wrote from a literal: the clears are
/// transcribed from the cache tool, the seed counters come from Incredibuild's
/// own statistics, and the ordering is checked against timestamps the runner
/// took around each build. Everything the ordering rests on is additionally
/// corroborated against the transcript file itself, because a timestamp only a
/// receipt asserts is a timestamp its writer chose.
fn verify_cache_chain(proof: &BuildProof, policy: &ProofPolicy) -> Result<CacheEvidence> {
    let source = &policy.transcripts;
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
    let mut citations = 0usize;
    let mut warm_seeds = 0usize;
    let mut ledger = TranscriptLedger::default();
    let mut tools: BTreeSet<String> = BTreeSet::new();

    for sample in &proof.samples {
        if sample.mode == "native" {
            continue;
        }
        let what = format!("{} sample {}", sample.mode, sample.repetition);
        if sample.cache_clears.is_empty() {
            bail!("{what} records no cache clear, so the cache it built against is unknown");
        }
        let mut cited: BTreeSet<PathBuf> = BTreeSet::new();
        for clear in &sample.cache_clears {
            citations += 1;
            // Every clear, not only the one `effective_clear` selects below.
            // A fabricated clear that finishes LATER shadows a real one and
            // would be caught as the effective clear; a fabricated EARLIER one
            // would sit in the receipt uncontested while the summary counted
            // it. A printed count must be a count of corroborated things.
            check_clear_usable(clear, &what)?;
            let (resolved, tool) = corroborate_clear(clear, &what, source, &mut ledger)?;
            // One sample citing one file twice says nothing twice. It is
            // either a bug in the runner or an attempt to make the evidence
            // look deeper than it is, and neither is something to summarize.
            // The counts printed below are taken from the ledger and so are
            // already immune to it; this refuses the receipt outright.
            if !cited.insert(resolved.clone()) {
                bail!(
                    "{what} cites transcript {} more than once; one file cited twice is one \
                     piece of evidence, not two",
                    resolved.display()
                );
            }
            if let Some(tool) = tool {
                tools.insert(tool);
            }
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
                match policy.distribution {
                    Distribution::Required if seed.remote_tasks == 0 => {
                        bail!("{what}: parent seed has no verified remote tasks");
                    }
                    Distribution::Excluded if seed.remote_tasks != 0 => {
                        bail!(
                            "{what}: parent seed reported {} remote task(s) under \
                             --distribution excluded; a cache-only profile declares rustc \
                             local_only, so nothing may have been sent to a helper",
                            seed.remote_tasks
                        );
                    }
                    _ => {}
                }
                if seed.cache_hits > policy.empty_cache_hit_floor {
                    bail!(
                        "{what}: parent seed reported {} cache hit(s) against a floor of {}, \
                         so it did not run against an emptied cache",
                        seed.cache_hits,
                        policy.empty_cache_hit_floor
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
    // Both counts are taken from the ledger, which holds one entry per FILE
    // actually opened in this invocation, so neither can be inflated by a
    // receipt that repeats itself.
    let anonymous_transcripts = ledger
        .read
        .values()
        .filter(|found| found.transcript.marker_version == 1)
        .count();
    Ok(CacheEvidence {
        scope,
        transcripts: ledger.read.len(),
        citations,
        warm_seeds,
        tools,
        anonymous_transcripts,
        source: source.describe(),
    })
}

fn validate_build_proof(proof: &BuildProof, policy: &ProofPolicy) -> Result<ProofSummary> {
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
            // Presence is demanded before either contract is applied, so that
            // `excluded` can never be satisfied by a counter that is simply
            // MISSING. An absent remote_tasks is unknown, not zero.
            let (Some(remote_tasks), Some(remote_core_time_s)) =
                (sample.remote_tasks, sample.remote_core_time_s)
            else {
                bail!(
                    "{} sample {} has no recorded remote-task counters",
                    sample.mode,
                    sample.repetition
                );
            };
            if sample.local_tasks.is_none() || sample.cache_misses.is_none() {
                bail!(
                    "{} sample {} has incomplete IB telemetry",
                    sample.mode,
                    sample.repetition
                );
            }
            match policy.distribution {
                Distribution::Required => {
                    if remote_tasks == 0 {
                        bail!(
                            "{} sample {} has no verified remote tasks",
                            sample.mode,
                            sample.repetition
                        );
                    }
                    if remote_core_time_s <= 0.0 {
                        bail!(
                            "{} sample {} has no verified remote core time",
                            sample.mode,
                            sample.repetition
                        );
                    }
                }
                // The cache-only contract. Stated as an equality against the
                // counters Incredibuild's own Build History reported, so a
                // cache-only claim is refused the moment anything reached a
                // helper -- including one stray task that would otherwise sit
                // unnoticed inside a "cache" result.
                Distribution::Excluded => {
                    if remote_tasks != 0 {
                        bail!(
                            "{} sample {} reported {} remote task(s) under --distribution \
                             excluded; a cache-only run distributes nothing",
                            sample.mode,
                            sample.repetition,
                            remote_tasks
                        );
                    }
                    if remote_core_time_s != 0.0 {
                        bail!(
                            "{} sample {} reported {}s of remote core time under \
                             --distribution excluded; a cache-only run distributes nothing",
                            sample.mode,
                            sample.repetition,
                            remote_core_time_s
                        );
                    }
                }
            }
        }
        // Cold and warm are judged against the same number, from opposite
        // sides: a cold build may not exceed the floor and a warm build must.
        // Raising the floor to excuse a cold sample therefore raises the bar
        // the warm samples have to clear, which is what stops it being a way
        // to make a receipt pass.
        if sample.mode == "ib-cold"
            && sample.cache_hits.unwrap_or(u64::MAX) > policy.empty_cache_hit_floor
        {
            bail!(
                "ib-cold sample {} was not empty-cache (hits={:?}, floor={})",
                sample.repetition,
                sample.cache_hits,
                policy.empty_cache_hit_floor
            );
        }
        if sample.mode == "ib-parent-warm"
            && sample.cache_hits.unwrap_or(0) <= policy.empty_cache_hit_floor
        {
            bail!(
                "ib-parent-warm sample {} has no verified cache hits above the empty-cache \
                 floor of {} (hits={:?}), so nothing distinguishes it from a cold build",
                sample.repetition,
                policy.empty_cache_hit_floor,
                sample.cache_hits
            );
        }
        by_mode.entry(sample.mode.clone()).or_default().push(sample);
    }

    // A build caption names ONE build to Incredibuild's Build History. Two
    // samples carrying one caption are therefore at most one build measured
    // twice, and the receipt does not say which measurement belongs to it.
    //
    // This is also the ONLY thing this invocation can establish about a
    // caption. `build-sample` checked, when the sample was recorded, that the
    // Build History held exactly one record for it reporting success; that
    // document is not retained, is not re-read here, and nothing printed below
    // claims otherwise.
    let mut captions: BTreeMap<&str, String> = BTreeMap::new();
    for sample in &proof.samples {
        let mut claimed: Vec<(&str, String)> = vec![(
            sample.build_caption.as_str(),
            format!("{} sample {}", sample.mode, sample.repetition),
        )];
        if let Some(seed) = &sample.parent_seed {
            claimed.push((
                seed.build_caption.as_str(),
                format!(
                    "the parent seed for {} sample {}",
                    sample.mode, sample.repetition
                ),
            ));
        }
        for (caption, owner) in claimed {
            if let Some(first) = captions.insert(caption, owner.clone()) {
                bail!(
                    "build caption {caption:?} is carried by both {first} and {owner}; one \
                     caption is one build, so these cannot both be measurements of their own"
                );
            }
        }
    }

    let mut stats = BTreeMap::new();
    for mode in ["native", "ib-cold", "ib-parent-warm"] {
        let samples = by_mode
            .get(mode)
            .with_context(|| format!("missing benchmark mode {mode}"))?;
        if samples.len() < policy.min_samples {
            bail!(
                "{mode} has {} sample(s), require at least {}",
                samples.len(),
                policy.min_samples
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
    let cache = verify_cache_chain(proof, policy)?;
    Ok(ProofSummary { stats, cache })
}

/// The three tiers `print_build_proof` prints, as functions returning the
/// text, so that the claims are values a test can read. The previous versions
/// existed only as literals inside `println!`, which is why two of them went
/// on asserting things the validator had never done: no test could see them.
///
/// Tier 1: what was RE-OBSERVED -- established in this invocation by opening a
/// file. Everything here is a statement about bytes on disk.
fn re_observed_paragraph(cache: &CacheEvidence) -> String {
    format!(
        "RE-OBSERVED IN THIS INVOCATION (transcripts resolved from {source}): {n} cache-clear \
         transcript file(s) were opened and read once, and the SHA-256 recomputed over exactly \
         the bytes that were then parsed matched the digest this receipt records, byte for \
         byte. Each file was re-parsed, and its argument vector, exit status, start and \
         completion instants, and the cache scope re-derived from its own arguments, all agree \
         with the fields this receipt carries -- for EVERY cache-clear record in the receipt, \
         not only the one the ordering selects. Where a transcript names its tool, its command \
         line is that tool followed by that argv. A receipt naming a file that is missing, that \
         has changed by one byte, or that contradicts it, is REFUSED here; it is not \
         summarized. This count is the number of FILES opened, taken from the ledger of reads, \
         not the number of records in the receipt.",
        source = cache.source,
        n = cache.transcripts
    )
}

/// Tier 2: what was CHECKED -- cross-field logic over numbers the receipt
/// asserts. Nothing in this tier opens a document, and nothing in it may
/// mention one. The Build History clause that used to live here was the
/// schema-v1 defect rebuilt in prose: a record-time check, performed on
/// another host by another invocation against a document this one cannot see,
/// printed under the word CHECKED. It is now in the tier below, where it is
/// true.
fn checked_paragraph(policy: &ProofPolicy) -> String {
    format!(
        "CHECKED FROM THIS RECEIPT'S OWN FIELDS: {min} sample(s) per mode carrying distinct \
         repetition numbers -- distinct is all that is shown, since the receipt's writer picks \
         the numbers; a distinct build caption for every sample and every parent seed; remote \
         task and remote core-time counters PRESENT for every IB sample (an absent counter is \
         unknown, never zero) and, under --distribution {distribution}, {distribution_rule}; \
         cold-cache and \
         parent-seed hits <={floor} and warm-cache hits >{floor} as recorded; for every ib-cold \
         sample, a local-user cache clear that exited 0 and completed before it with no \
         cache-using build in between; for every ib-parent-warm sample, that same ordering \
         between the clear and its PARENT SEED, and no cache-using build between that seed and \
         the measured build -- the seed itself ran between the clear and the measured build, \
         which is what warming a cache means. This is cross-field logic over numbers the \
         receipt asserts: no document is opened for any of it. The cache-clear fields the \
         ordering rests on are corroborated above; every counter it compares is not.",
        min = policy.min_samples,
        floor = policy.empty_cache_hit_floor,
        distribution = policy.distribution,
        distribution_rule = match policy.distribution {
            Distribution::Required =>
                "every IB sample and every parent seed reporting remote_tasks > 0 and \
                 remote_core_time > 0",
            Distribution::Excluded =>
                "every IB sample and every parent seed reporting remote_tasks == 0 and \
                 remote_core_time == 0, so nothing in this receipt was distributed",
        }
    )
}

/// Tier 3: what is NOT CHECKED. This is the tier that has to be complete,
/// because everything a reader is entitled to distrust has to be findable
/// here. It names the counters, the clock, the wall times and the signature
/// that is absent -- and the record-time checks that happened elsewhere.
fn not_checked_paragraph() -> String {
    "NOT CHECKED: the transcripts are corroborated; the builds are not. No document behind any \
     counter in this receipt is retained with a digest or re-read here -- not the Build History \
     responses behind remote_tasks and local_tasks, not the cache-statistics output behind \
     cache_hits and cache_misses, not the per-task Build Cache report, and not one of the \
     parent-seed counters, which is what the attribution of a warm cache to its parent rests \
     on. `build-sample` did check, WHEN EACH SAMPLE WAS RECORDED, that the Build History held \
     exactly one record for that caption reporting success; that happened in another \
     invocation, on another host, against a document this one cannot see, and this invocation \
     re-establishes none of it. Neither is wall_ms, from which every ratio above is computed: \
     nothing retains it and nothing cross-checks it. swf-cli does not observe the cache: a \
     transcript shows that the named tool ran and what it printed, not that the namespace was \
     empty, and nothing here rules out another process repopulating it between the clear and \
     the build. Every timestamp is the runner's own clock, unattested, and nothing binds a \
     transcript to the machine, user or filesystem that ran the build. Neither the receipt nor \
     the transcripts are signed, so a party able to write both can still make them agree -- \
     including the tool names printed above, which are read from those same transcripts. \
     Re-verify independently before quoting any ratio."
        .to_string()
}

fn print_build_proof(proof: &BuildProof, policy: &ProofPolicy) -> Result<()> {
    let ProofSummary { stats, cache } = validate_build_proof(proof, policy)?;
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
    // Printed on EVERY run, both settings, because "this was a cache-only
    // measurement" and "this was a distribution measurement" are different
    // claims and a reader must not have to infer which one a summary supports.
    match policy.distribution {
        Distribution::Required => println!(
            "DISTRIBUTION CONTRACT: --distribution required. Every IB sample and parent seed \
             in this receipt reports remote_tasks > 0 and remote_core_time > 0, so work \
             crossed the network. The wall times above therefore mix distribution and \
             caching, and this summary does NOT separate them."
        ),
        Distribution::Excluded => println!(
            "DISTRIBUTION CONTRACT: --distribution excluded (CACHE-ONLY). Every IB sample and \
             parent seed in this receipt reports remote_tasks == 0 and remote_core_time == 0 \
             as recorded, i.e. nothing was sent to a helper; the expected profile is rustc \
             type=\"local_only\" with <ib_cache enabled=\"true\"/>. Any acceleration above is \
             therefore the Build Cache's and not distribution's. This is a demand, not a \
             waiver: one remote task would have REFUSED this receipt. What is NOT established \
             here is that the profile actually said local_only -- these are the Build History \
             counters the receipt carries, and like every other counter in it they are \
             re-read from no retained document (see NOT CHECKED below)."
        ),
    }
    println!(
        "cache scope re-derived from {} corroborated clear transcript file(s): {}",
        cache.transcripts, cache.scope
    );
    // A receipt chooses how often it cites a file. Printing both numbers means
    // it can inflate the one it controls only by making the gap visible.
    if cache.citations != cache.transcripts {
        println!(
            "those {} file(s) are cited by {} cache-clear record(s) in this receipt",
            cache.transcripts, cache.citations
        );
    }
    println!(
        "cache tool(s) named by those transcripts: {}",
        if cache.tools.is_empty() {
            "none -- no transcript named the tool that ran".to_string()
        } else {
            cache.tools.iter().cloned().collect::<Vec<_>>().join(", ")
        }
    );
    // Deliberately on its own line and not in the corroboration sentence: a
    // parent seed is a set of counters and a start time that only the receipt
    // asserts, and standing next to the word "corroborated" was lending it
    // standing it has not got.
    println!(
        "{} parent-seed build(s) attributed to the parent revision ON THIS RECEIPT'S WORD \
         ALONE: no document behind a seed's counters is retained or re-read here",
        cache.warm_seeds
    );
    if cache.anonymous_transcripts > 0 {
        println!(
            "{} of those file(s) were written by the v1 recorder, which shifted the tool \
             name off before recording argv; WHICH TOOL emptied the cache is not established \
             for them. Re-run with scripts/robot-demo/cache-clear.sh, which emits {}.",
            cache.anonymous_transcripts, CACHE_CLEAR_MARKERS[1]
        );
    }
    if policy.empty_cache_hit_floor > 0 {
        println!(
            "EMPTY-CACHE HIT FLOOR: this check ran with --empty-cache-hit-floor {floor}, which \
             the VERIFIER passed on the command line and the receipt cannot set. An emptied \
             cache was allowed to report up to {floor} Build Cache hit(s), and a warm build was \
             required to report more than {floor}. cargo invokes `rustc -vV` twice per build \
             and the second invocation is served the entry the first stored, which is why the \
             honest floor for Rust is not zero. NOTHING HERE ESTABLISHES that the hits this \
             floor allowed were that self-hit: the per-task Build Cache report that would show \
             it is not retained with a digest. Run without the flag for the strict rule.",
            floor = policy.empty_cache_hit_floor
        );
    }
    println!("{}", re_observed_paragraph(&cache));
    println!("{}", checked_paragraph(policy));
    println!("{}", not_checked_paragraph());
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
                transcripts,
                empty_cache_hit_floor,
                distribution,
            } => {
                let source = match transcripts {
                    None => TranscriptSource::as_recorded(),
                    Some(dir) => {
                        if !dir.is_dir() {
                            bail!("--transcripts {} is not a directory", dir.display());
                        }
                        TranscriptSource::relocated(dir)
                    }
                };
                let policy = ProofPolicy::checking(min_samples, source)
                    .with_empty_cache_hit_floor(empty_cache_hit_floor)
                    .with_distribution(Distribution::parse(&distribution)?);
                let proof: BuildProof = serde_json::from_str(
                    &std::fs::read_to_string(&receipt)
                        .with_context(|| format!("reading {}", receipt.display()))?,
                )
                .with_context(|| format!("parsing {}", receipt.display()))?;
                print_build_proof(&proof, &policy)?;
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

    // --- real evidence for the synthetic receipts ------------------------
    //
    // Every `CacheClear` below is produced by `load_cache_clear` from a
    // transcript these tests actually wrote to disk, so its path is a real
    // path and its digest is a real digest of real bytes. That is deliberate
    // and it is the whole point: a fixture that handed the validator a
    // literal digest would only ever exercise the code path that reads
    // literals, which is the defect this change exists to close.
    //
    // There is deliberately NO test-only bypass -- no `skip`, no
    // `cfg(test)` exemption, no "assume valid" constructor. Such a thing
    // would be the schema-v1 tautology rebuilt one layer down, dormant in
    // the binary, and it would be the single code path most of these tests
    // ran through. `corroboration_has_no_opt_out` below fails if one appears.

    /// A directory that lives as long as the fixture and removes itself.
    /// Deliberately not the `tempfile` crate: this workspace's dependency
    /// graph IS the compilation workload the benchmark measures, so the test
    /// suite must not grow it.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock is before the epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "swf-cli-{tag}-{}-{nanos}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const TEST_CACHE_TOOL: &str = "/usr/local/bin/ib_console_cache";

    /// The grammar scripts/robot-demo/cache-clear.sh emits for v2.
    fn transcript_text(
        tool: &str,
        argv: &str,
        exit_code: i32,
        started: u64,
        completed: u64,
    ) -> String {
        format!(
            "{marker}\ntool={tool}\ncommand={tool} {argv}\nargv={argv}\nexit_code={exit_code}\n\
             started_at_ms={started}\ncompleted_at_ms={completed}\n\
             --- transcript ---\nLocal user build cache cleared.\n",
            marker = CACHE_CLEAR_MARKERS[1]
        )
    }

    /// Write a transcript and return its path, without going near a receipt.
    /// Used directly only where a test needs a file the recording path would
    /// refuse to produce (a non-zero exit, a v1 header).
    fn transcript_file(
        dir: &TempDir,
        argv: &str,
        exit_code: i32,
        started: u64,
        completed: u64,
    ) -> PathBuf {
        let slug: String = argv
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let path = dir.path().join(format!(
            "clear-{started}-{completed}-{exit_code}-{slug}.txt"
        ));
        std::fs::write(
            &path,
            transcript_text(TEST_CACHE_TOOL, argv, exit_code, started, completed),
        )
        .expect("write transcript");
        path
    }

    /// Write a genuine transcript and return the `CacheClear` that the
    /// PRODUCTION loader parses out of it -- the same call ib-benchmark.sh
    /// makes. This is the only way a fixture obtains a `CacheClear`.
    fn write_clear(dir: &TempDir, argv: &str, started: u64, completed: u64) -> CacheClear {
        let path = transcript_file(dir, argv, 0, started, completed);
        load_cache_clear(&path).expect("a transcript this fixture just wrote must load")
    }

    /// A deterministic timeline, one second-scale block per repetition:
    /// native at +100, the cold clear at +200 and its build at +300, the warm
    /// clear at +400, its parent seed at +500 and the measured warm build at
    /// +600. Every cache-using build gets a distinct start.
    fn proof_sample(
        dir: &TempDir,
        mode: &str,
        repetition: usize,
        remote_tasks: Option<u64>,
        cache_hits: Option<u64>,
    ) -> BuildSample {
        let base = repetition as u64 * 1_000_000;
        let (started_at_ms, cache_clears, parent_seed) = match mode {
            "native" => (base + 100, Vec::new(), None),
            "ib-cold" => (
                base + 300,
                vec![write_clear(dir, "user clear", base + 190, base + 200)],
                None,
            ),
            _ => (
                base + 600,
                vec![write_clear(dir, "user clear", base + 390, base + 400)],
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

    /// A receipt plus the directory holding the transcripts it cites. The two
    /// travel together because the receipt is meaningless without them --
    /// which is exactly what this change made true.
    struct Fixture {
        dir: TempDir,
        proof: BuildProof,
    }

    fn complete_build_proof() -> Fixture {
        let dir = TempDir::new("proof");
        let mut samples = Vec::new();
        for repetition in 1..=5 {
            samples.push(proof_sample(&dir, "native", repetition, None, None));
            samples.push(proof_sample(&dir, "ib-cold", repetition, Some(4), Some(0)));
            samples.push(proof_sample(
                &dir,
                "ib-parent-warm",
                repetition,
                Some(1),
                Some(3),
            ));
        }
        Fixture {
            dir,
            proof: BuildProof {
                schema_version: BUILD_PROOF_SCHEMA_VERSION,
                run_id: "proof-1".into(),
                candidate_revision: "candidate".into(),
                parent_revision: "parent".into(),
                samples,
            },
        }
    }

    /// Validate exactly as `build-proof` does, with the transcripts resolved
    /// at the paths the receipt records.
    fn validate(fixture: &Fixture) -> Result<ProofSummary> {
        validate_build_proof(&fixture.proof, &strictly(TranscriptSource::as_recorded()))
    }

    /// The default standard these tests hold a receipt to: five samples per
    /// mode, transcripts wherever the argument says, and an emptied cache that
    /// served nothing. Nothing here is loosened for the tests' convenience.
    fn strictly(transcripts: TranscriptSource) -> ProofPolicy {
        ProofPolicy::checking(5, transcripts)
    }

    fn accepted(fixture: &Fixture) -> ProofSummary {
        validate(fixture).expect("expected this receipt to be accepted")
    }

    fn rejection(fixture: &Fixture) -> String {
        let error = validate(fixture).expect_err("expected this receipt to be rejected");
        // Alternate Display, so `with_context` layers are visible to the
        // substring assertions below rather than only the outermost message.
        format!("{error:#}")
    }

    fn warm_mut(proof: &mut BuildProof) -> &mut BuildSample {
        proof
            .samples
            .iter_mut()
            .find(|sample| sample.mode == "ib-parent-warm")
            .unwrap()
    }

    /// Turn a distribution fixture into the CACHE-ONLY one: every remote
    /// counter zeroed, on the samples and on the parent seeds alike, exactly
    /// as Incredibuild's Build History reports a build that sent nothing to a
    /// helper. Every cache counter is left untouched.
    fn as_cache_only(proof: &mut BuildProof) {
        for sample in &mut proof.samples {
            if sample.mode == "native" {
                continue;
            }
            sample.remote_tasks = Some(0);
            sample.remote_core_time_s = Some(0.0);
            if let Some(seed) = sample.parent_seed.as_mut() {
                seed.remote_tasks = 0;
            }
        }
    }

    fn validate_cache_only(fixture: &Fixture) -> Result<ProofSummary> {
        validate_build_proof(
            &fixture.proof,
            &strictly(TranscriptSource::as_recorded()).with_distribution(Distribution::Excluded),
        )
    }

    /// The cache-only contract is a DEMAND, not a waiver of the distribution
    /// one. Both directions are pinned here, because the whole defence against
    /// "switch to --distribution excluded and the receipt passes" is that the
    /// switch immediately forbids what it was reached for.
    #[test]
    fn the_cache_only_contract_forbids_distribution_rather_than_ignoring_it() {
        let mut fixture = complete_build_proof();
        // The distribution receipt is REFUSED by the cache-only contract:
        // its samples really did send work to helpers.
        let refused = format!(
            "{:#}",
            validate_cache_only(&fixture).expect_err("distributed receipt must fail cache-only")
        );
        assert!(refused.contains("--distribution excluded"), "{refused}");

        as_cache_only(&mut fixture.proof);
        // ... and symmetrically, the cache-only receipt is REFUSED by the
        // default contract, so neither setting is the lenient one.
        assert!(
            rejection(&fixture).contains("no verified remote tasks"),
            "cache-only receipt must fail the default distribution contract"
        );
        validate_cache_only(&fixture).expect("a zero-remote receipt satisfies cache-only");

        // One leaked task is a rejection, which is what makes "nothing was
        // distributed" a finding rather than a formatting choice.
        fixture.proof.samples[1].remote_tasks = Some(1);
        let leaked = format!(
            "{:#}",
            validate_cache_only(&fixture).expect_err("one remote task must refuse cache-only")
        );
        assert!(leaked.contains("reported 1 remote task(s)"), "{leaked}");
        fixture.proof.samples[1].remote_tasks = Some(0);

        // So is a leak recorded only as core time.
        fixture.proof.samples[1].remote_core_time_s = Some(0.25);
        let core = format!(
            "{:#}",
            validate_cache_only(&fixture).expect_err("remote core time must refuse cache-only")
        );
        assert!(core.contains("remote core time"), "{core}");
        fixture.proof.samples[1].remote_core_time_s = Some(0.0);

        // A parent seed that distributed is a leak too: the warm cache it
        // built would have been populated with help from the network.
        warm_mut(&mut fixture.proof)
            .parent_seed
            .as_mut()
            .unwrap()
            .remote_tasks = 3;
        let seed = format!(
            "{:#}",
            validate_cache_only(&fixture).expect_err("a distributed seed must refuse cache-only")
        );
        assert!(
            seed.contains("parent seed reported 3 remote task(s)"),
            "{seed}"
        );
    }

    /// Every CACHE check survives the switch to cache-only. If this ever
    /// stopped holding, `--distribution excluded` would have become a way to
    /// get a weaker receipt accepted.
    #[test]
    fn the_cache_only_contract_leaves_every_cache_check_in_force() {
        let mut fixture = complete_build_proof();
        as_cache_only(&mut fixture.proof);
        validate_cache_only(&fixture).expect("baseline cache-only receipt");

        warm_mut(&mut fixture.proof).cache_hits = Some(0);
        let flat = format!("{:#}", validate_cache_only(&fixture).unwrap_err());
        assert!(flat.contains("no verified cache hits"), "{flat}");
        warm_mut(&mut fixture.proof).cache_hits = Some(3);

        fixture.proof.samples[1].cache_hits = Some(7);
        let warm_cold = format!("{:#}", validate_cache_only(&fixture).unwrap_err());
        assert!(warm_cold.contains("was not empty-cache"), "{warm_cold}");
        fixture.proof.samples[1].cache_hits = Some(0);

        // And the clear transcripts are still opened, re-hashed and re-parsed.
        fixture.proof.samples[1].cache_clears.clear();
        let unclear = format!("{:#}", validate_cache_only(&fixture).unwrap_err());
        assert!(
            unclear.contains("cache clear") || unclear.contains("clear"),
            "{unclear}"
        );
    }

    /// An ABSENT remote counter is unknown, and unknown is not zero. Without
    /// this, a receipt that simply omitted its remote telemetry would satisfy
    /// the cache-only contract by saying nothing at all.
    #[test]
    fn a_missing_remote_counter_never_satisfies_the_cache_only_contract() {
        let mut fixture = complete_build_proof();
        as_cache_only(&mut fixture.proof);
        fixture.proof.samples[1].remote_tasks = None;
        let message = format!("{:#}", validate_cache_only(&fixture).unwrap_err());
        assert!(
            message.contains("no recorded remote-task counters"),
            "{message}"
        );

        fixture.proof.samples[1].remote_tasks = Some(0);
        fixture.proof.samples[1].remote_core_time_s = None;
        let message = format!("{:#}", validate_cache_only(&fixture).unwrap_err());
        assert!(
            message.contains("no recorded remote-task counters"),
            "{message}"
        );
    }

    #[test]
    fn build_proof_requires_distribution_and_cache_hits() {
        let mut fixture = complete_build_proof();
        assert!(validate(&fixture).is_ok());

        fixture.proof.samples[1].remote_tasks = Some(0);
        assert!(rejection(&fixture).contains("no verified remote tasks"));
        fixture.proof.samples[1].remote_tasks = Some(4);

        warm_mut(&mut fixture.proof).cache_hits = Some(0);
        assert!(rejection(&fixture).contains("no verified cache hits"));
    }

    #[test]
    fn build_proof_requires_five_samples_per_mode_with_distinct_repetitions() {
        let mut fixture = complete_build_proof();
        fixture
            .proof
            .samples
            .retain(|sample| sample.mode != "native" || sample.repetition != 5);
        assert!(rejection(&fixture).contains("require at least 5"));

        // Five records is not five samples unless they are five different
        // ones, and a repetition number is the only thing in the receipt that
        // says which sample a record is. Two records claiming to be the same
        // repetition are one measurement counted twice -- which is all this
        // check establishes. It does not make them independent; the receipt's
        // writer picks the numbers, and the summary says only "distinct".
        let mut fixture = complete_build_proof();
        let stolen = fixture.proof.samples[0].repetition;
        fixture
            .proof
            .samples
            .iter_mut()
            .find(|sample| sample.mode == "native" && sample.repetition != stolen)
            .unwrap()
            .repetition = stolen;
        assert!(rejection(&fixture).contains("duplicate repetition numbers"));
    }

    // --- the honesty regression suite -----------------------------------
    //
    // Schema 1 stamped the cache scope and the clearing procedure as literals
    // at receipt-assembly time and then validated those same literals, so the
    // three corresponding bails could never fire. Each test below drives one
    // of the replacements, which read only records the runner produced.

    #[test]
    fn schema_one_receipts_are_rejected_because_their_attestations_were_literals() {
        let mut fixture = complete_build_proof();
        fixture.proof.schema_version = 1;
        let message = rejection(&fixture);
        assert!(message.contains("schema 1"), "{message}");
        assert!(message.contains("literals"), "{message}");
    }

    #[test]
    fn an_ib_sample_without_an_observed_clear_is_unknown_not_clean() {
        let mut fixture = complete_build_proof();
        fixture.proof.samples[1].cache_clears.clear();
        assert!(rejection(&fixture).contains("records no cache clear"));

        let mut fixture = complete_build_proof();
        warm_mut(&mut fixture.proof).cache_clears.clear();
        assert!(rejection(&fixture).contains("records no cache clear"));
    }

    /// These four drive `check_clear_usable`, which reads the receipt's own
    /// fields and runs BEFORE the transcript is opened. The receipt is
    /// therefore mutated in place on purpose: the point is that a receipt
    /// which is self-evidently unusable is refused without any filesystem
    /// work, and that each of those four bails still fires exactly as it did
    /// before corroboration existed.
    #[test]
    fn a_clear_is_rejected_unless_it_is_local_user_successful_and_retained() {
        let mut fixture = complete_build_proof();
        fixture.proof.samples[1].cache_clears[0].scope = CacheScope::Shared;
        assert!(rejection(&fixture).contains("shared cache"));

        let mut fixture = complete_build_proof();
        fixture.proof.samples[1].cache_clears[0].scope = CacheScope::Unknown;
        assert!(rejection(&fixture).contains("unrecognized scope"));

        let mut fixture = complete_build_proof();
        fixture.proof.samples[1].cache_clears[0].exit_code = 3;
        assert!(rejection(&fixture).contains("exited 3"));

        let mut fixture = complete_build_proof();
        fixture.proof.samples[1].cache_clears[0].transcript_sha256 = "not-a-digest".into();
        assert!(rejection(&fixture).contains("no retained transcript digest"));
    }

    #[test]
    fn a_clear_must_precede_the_build_it_is_claimed_to_have_prepared() {
        let mut fixture = complete_build_proof();
        // A GENUINE transcript that simply finished one millisecond after the
        // timed build began. Backdating the receipt's field instead would now
        // be caught by corroboration, which is a different bail; the ordering
        // rule has to be shown firing on evidence that is itself sound.
        let start = fixture.proof.samples[1].started_at_ms;
        let late = write_clear(&fixture.dir, "user clear", start - 10, start + 1);
        fixture.proof.samples[1].cache_clears = vec![late];
        assert!(rejection(&fixture).contains("finished after the build started"));
    }

    #[test]
    fn a_clear_cannot_be_reused_across_an_intervening_cache_using_build() {
        let mut fixture = complete_build_proof();
        // Repetition 2's cold clear genuinely happened ahead of repetition 1's
        // warm build, so another Incredibuild build ran in between and could
        // have repopulated the cache.
        let early = write_clear(&fixture.dir, "user clear", 1_000_540, 1_000_550);
        let cold_two = fixture
            .proof
            .samples
            .iter_mut()
            .find(|sample| sample.mode == "ib-cold" && sample.repetition == 2)
            .unwrap();
        cold_two.cache_clears = vec![early];
        assert!(rejection(&fixture).contains("ran between the cache clear"));
    }

    #[test]
    fn warm_cache_hits_must_be_attributable_to_a_recorded_parent_seed() {
        let mut fixture = complete_build_proof();
        warm_mut(&mut fixture.proof).parent_seed = None;
        assert!(rejection(&fixture).contains("records no parent seed"));

        let mut fixture = complete_build_proof();
        warm_mut(&mut fixture.proof)
            .parent_seed
            .as_mut()
            .unwrap()
            .cache_hits = 2;
        assert!(rejection(&fixture).contains("did not run against an emptied cache"));

        let mut fixture = complete_build_proof();
        warm_mut(&mut fixture.proof)
            .parent_seed
            .as_mut()
            .unwrap()
            .source_revision = "candidate".into();
        assert!(rejection(&fixture).contains("expected parent"));

        let mut fixture = complete_build_proof();
        warm_mut(&mut fixture.proof)
            .parent_seed
            .as_mut()
            .unwrap()
            .remote_tasks = 0;
        assert!(rejection(&fixture).contains("parent seed has no verified remote tasks"));
    }

    #[test]
    fn native_samples_may_not_carry_cache_evidence() {
        let mut fixture = complete_build_proof();
        let stray = write_clear(&fixture.dir, "user clear", 40, 50);
        fixture.proof.samples[0].cache_clears.push(stray);
        assert!(rejection(&fixture).contains("IB-only telemetry"));
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

    /// The Incredibuild 4.30 spellings, pinned against a record copied from
    /// this grid's own Build History API.
    ///
    /// These three adapters were once fixed on the initiator and then lost
    /// when a newer copy of this file was synced over the top of them, and the
    /// loss was silent: every substantive check still compiled and every other
    /// test still passed, while the gate could no longer find a single build.
    /// A test is the only thing that makes that kind of drift loud.
    #[test]
    fn the_history_parser_reads_incredibuild_4_30_spellings() {
        let record = serde_json::json!({
            "builds": [{
                "buildId": "uuid_ec23ccb6-591f-d8ca-65dc-e034d646a4f0_buildid_186292_000056",
                "buildTitle": "cacheonly-smoke-ib-cold-1",
                "buildStatus": "Succeeded",
                "numberOfLocalTasks": 60,
                "numberOfRemoteTasks": 0,
                "remoteCoreTime": 0
            }]
        });
        let parsed = parse_ib_history(&record, "cacheonly-smoke-ib-cold-1")
            .expect("4.30 reports the caption as buildTitle and the status as Succeeded");
        // The initiator-local build number is the buildId's last segment: it
        // is what names incredibuildBuildReport_<n>.db, which is what
        // show_build_cache_statistics.sh is keyed by.
        assert_eq!(parsed.build_number, 56);
        // A cache-only build. Zero is a VALUE here, not an absence, and the
        // parser must carry it through rather than reject it -- the cache-only
        // contract in build-proof is what decides whether zero is acceptable.
        assert_eq!(parsed.remote_tasks, 0);
        assert_eq!(parsed.remote_core_time_s, 0.0);
        assert_eq!(parsed.local_tasks, 60);

        // A buildId with no numeric tail is an error, never a guess.
        let opaque = serde_json::json!({
            "builds": [{
                "buildId": "uuid_abc_buildid_nope",
                "buildTitle": "x",
                "buildStatus": "Succeeded",
                "numberOfLocalTasks": 1,
                "numberOfRemoteTasks": 0,
                "remoteCoreTime": 0
            }]
        });
        assert!(parse_ib_history(&opaque, "x").is_err());
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

    // --- the forgery regression suite ------------------------------------
    //
    // Failure #1 was a receipt validating its own literals. The fix moved the
    // hole rather than closing it: schema v2 introduced cache-clear
    // transcripts, and the validator then checked that `transcript_sha256`
    // LOOKED like a digest and that `transcript_path` was non-empty. It never
    // opened the file. Every test below is a receipt that passed that
    // validator and must not pass this one.

    /// The receipt that was fabricated today. Five real native samples, ten
    /// Incredibuild samples typed by hand with an invented `remote_tasks` and
    /// a cache clear citing `/tmp/does-not-exist.txt` with a digest of
    /// sixty-four zeros. It printed
    ///
    /// ```text
    /// BUILD RECEIPT CONSISTENT
    /// ib-parent-warm  measured ratio=11.948x vs native; saved=7116ms
    /// ```
    ///
    /// and exited 0. Nothing in it was invented but JSON fields.
    #[test]
    fn the_receipt_that_printed_ratio_11_948x_from_invented_fields_is_now_refused() {
        let ghost = "/tmp/does-not-exist.txt";
        let mut fixture = complete_build_proof();
        for sample in &mut fixture.proof.samples {
            if sample.mode == "native" {
                continue;
            }
            sample.remote_tasks = Some(412);
            sample.remote_core_time_s = Some(0.001);
            for clear in &mut sample.cache_clears {
                clear.transcript_path = ghost.into();
                clear.transcript_sha256 = "0".repeat(64);
            }
        }

        // Sixty-four zeros are well-formed lowercase hex and a non-empty path
        // is a non-empty path, so every check that reads only the receipt's
        // own fields still passes. That is precisely why the refusal has to
        // come from the filesystem and cannot come from the receipt.
        assert!(
            check_clear_usable(&fixture.proof.samples[1].cache_clears[0], "the forgery").is_ok(),
            "the forged fields are well shaped; only a file can contradict them"
        );

        let message = rejection(&fixture);
        assert!(message.contains("does-not-exist"), "{message}");
        // If another process really has left that path behind, the forgery is
        // still refused -- by the digest instead of by the missing file.
        let expected = if Path::new(ghost).exists() {
            "does not describe this file"
        } else {
            "not present at its recorded path"
        };
        assert!(message.contains(expected), "{message}");

        // And it never reaches a printed ratio again: build-proof returns
        // Err, which main turns into a non-zero exit.
        assert!(
            print_build_proof(&fixture.proof, &strictly(TranscriptSource::as_recorded())).is_err(),
            "this receipt must never print a measured ratio again"
        );
    }

    #[test]
    fn a_transcript_digest_that_does_not_match_the_file_is_refused() {
        // The file exists and the digest is well formed. They are simply
        // about different bytes, which makes the receipt evidence about some
        // other file and therefore no evidence about this one.
        let mut fixture = complete_build_proof();
        fixture.proof.samples[1].cache_clears[0].transcript_sha256 = "0".repeat(64);
        let message = rejection(&fixture);
        assert!(message.contains("hashes to"), "{message}");
        assert!(message.contains("does not describe this file"), "{message}");
    }

    #[test]
    fn a_transcript_edited_after_the_receipt_was_written_is_refused() {
        let fixture = complete_build_proof();
        let path = PathBuf::from(&fixture.proof.samples[1].cache_clears[0].transcript_path);
        let text = std::fs::read_to_string(&path).unwrap();
        // One trailing space. It still parses to exactly the same fields, and
        // it is no longer the file the receipt was written about.
        std::fs::write(&path, format!("{text} ")).unwrap();
        let message = rejection(&fixture);
        assert!(message.contains("hashes to"), "{message}");
        assert!(message.contains("does not describe this file"), "{message}");
    }

    #[test]
    fn a_receipt_that_contradicts_the_transcript_on_disk_is_refused() {
        // (a) The instants the whole cache-ordering argument rests on.
        let mut fixture = complete_build_proof();
        fixture.proof.samples[1].cache_clears[0].started_at_ms += 1;
        let message = rejection(&fixture);
        assert!(message.contains("disagrees with transcript"), "{message}");
        assert!(message.contains("started_at_ms"), "{message}");

        let mut fixture = complete_build_proof();
        fixture.proof.samples[1].cache_clears[0].completed_at_ms += 1;
        assert!(rejection(&fixture).contains("completed_at_ms"));

        // (b) The arguments that decide which cache namespace was emptied.
        let mut fixture = complete_build_proof();
        fixture.proof.samples[1].cache_clears[0].argv = "user clear --force".into();
        assert!(rejection(&fixture).contains("argv"));

        // (c) The receipt claims the local-user namespace while the
        // transcript's own arguments say the shared one was emptied. The
        // receipt's `scope` passes check_clear_usable precisely because
        // local-user is the value a forger would choose; the scope re-derived
        // from the file is what disagrees.
        let mut fixture = complete_build_proof();
        let mut forged = write_clear(&fixture.dir, "shared clear", 1_000_190, 1_000_200);
        forged.scope = CacheScope::LocalUser;
        fixture.proof.samples[1].cache_clears = vec![forged];
        let message = rejection(&fixture);
        assert!(message.contains("disagrees with transcript"), "{message}");
        assert!(message.contains("scope"), "{message}");
    }

    #[test]
    fn a_receipt_claiming_exit_zero_over_a_transcript_recording_failure_is_refused() {
        // The recording path refuses a failed clear outright, so a transcript
        // with a non-zero exit behind a receipt claiming 0 can only have been
        // assembled by hand. `parse_clear_transcript` REPORTS the exit status
        // instead of rejecting it so that proof time can name the
        // disagreement rather than lose it inside a parse error.
        let mut fixture = complete_build_proof();
        let real = fixture.proof.samples[1].cache_clears[0].clone();
        let path = transcript_file(
            &fixture.dir,
            "user clear",
            3,
            real.started_at_ms,
            real.completed_at_ms,
        );
        assert!(
            load_cache_clear(&path).is_err(),
            "the recording path must still refuse to record a failed clear"
        );
        fixture.proof.samples[1].cache_clears = vec![CacheClear {
            scope: CacheScope::LocalUser,
            argv: "user clear".into(),
            exit_code: 0,
            started_at_ms: real.started_at_ms,
            completed_at_ms: real.completed_at_ms,
            transcript_path: path.display().to_string(),
            // A real digest of the real file: the binding holds, and the
            // file still contradicts the claim.
            transcript_sha256: evidence::sha256_file(&path).unwrap(),
        }];
        let message = rejection(&fixture);
        assert!(message.contains("disagrees with transcript"), "{message}");
        assert!(
            message.contains("exit_code (receipt 0, transcript 3)"),
            "{message}"
        );
    }

    #[test]
    fn every_clear_is_corroborated_not_only_the_one_the_ordering_selects() {
        // `effective_clear` picks the LATEST clear at or before the build, so
        // a fabricated EARLIER one is never selected. It used to sit in the
        // receipt uncontested while the summary counted it as observed.
        let mut fixture = complete_build_proof();
        let real = fixture.proof.samples[1].cache_clears[0].clone();
        let ghost = CacheClear {
            scope: CacheScope::LocalUser,
            argv: "user clear".into(),
            exit_code: 0,
            started_at_ms: real.started_at_ms - 100,
            completed_at_ms: real.completed_at_ms - 100,
            transcript_path: fixture
                .dir
                .path()
                .join("this-file-has-never-existed.txt")
                .display()
                .to_string(),
            transcript_sha256: "0".repeat(64),
        };
        fixture.proof.samples[1].cache_clears = vec![ghost, real];
        let message = rejection(&fixture);
        assert!(message.contains("this-file-has-never-existed"), "{message}");
    }

    #[test]
    fn the_printed_clear_count_counts_transcripts_opened_in_this_invocation() {
        let fixture = complete_build_proof();
        let summary = accepted(&fixture);
        // Ten Incredibuild samples, one clear each, every one of them opened,
        // re-hashed and re-parsed.
        assert_eq!(summary.cache.transcripts, 10);
        assert_eq!(summary.cache.citations, 10);
        assert_eq!(summary.cache.warm_seeds, 5);
        assert_eq!(summary.cache.scope, CacheScope::LocalUser);
        assert_eq!(summary.cache.anonymous_transcripts, 0);
        assert!(
            print_build_proof(&fixture.proof, &strictly(TranscriptSource::as_recorded())).is_ok()
        );

        // The fixture above gives every sample its own transcript, so a count
        // of files and a count of JSON records are the same number and this
        // test could not tell the two readings apart -- which is exactly how
        // a counter over the receipt's own records survived here while the
        // summary called its output "corroborated clear transcript(s)".
        //
        // So: a second sample additionally cites a transcript that already
        // exists and has already been corroborated, for a clear that finished
        // long before its own. Eleven records now cite ten files. The printed
        // number must follow the files.
        let mut fixture = complete_build_proof();
        let already_counted = fixture.proof.samples[1].cache_clears[0].clone();
        assert_eq!(fixture.proof.samples[4].mode, "ib-cold");
        fixture.proof.samples[4]
            .cache_clears
            .push(already_counted.clone());
        let summary = accepted(&fixture);
        assert_eq!(
            summary.cache.transcripts, 10,
            "ten files exist and ten were opened, whatever the receipt cites"
        );
        assert_eq!(
            summary.cache.citations, 11,
            "the citation count is reported separately, so inflating it is visible"
        );

        // One genuine transcript cited five hundred times would otherwise have
        // printed "500 corroborated clear transcript(s)" over one file.
        let mut fixture = complete_build_proof();
        for _ in 0..500 {
            fixture.proof.samples[4]
                .cache_clears
                .push(already_counted.clone());
        }
        let message = rejection(&fixture);
        assert!(message.contains("more than once"), "{message}");
    }

    #[test]
    fn a_sample_may_not_cite_one_transcript_twice() {
        // Repeating a citation says one thing twice. The counts above are
        // already immune to it; the receipt is refused anyway, because a
        // runner does not produce this and the only reason to write it is to
        // make the evidence look deeper than it is.
        let mut fixture = complete_build_proof();
        let own = fixture.proof.samples[1].cache_clears[0].clone();
        fixture.proof.samples[1].cache_clears.push(own);
        let message = rejection(&fixture);
        assert!(message.contains("more than once"), "{message}");
        assert!(
            message.contains("not two"),
            "the refusal must say why one file cited twice is one piece of evidence: {message}"
        );
    }

    #[test]
    fn two_builds_may_not_share_one_build_caption() {
        // A caption names ONE build to Incredibuild's Build History. Fifteen
        // samples under one caption are at most one build measured fifteen
        // times, and this receipt printed a ratio over them: every sample
        // carrying "the-one-and-only-caption" validated and exited 0, because
        // the only thing the validator ever asked of a caption was that it was
        // not the empty string.
        let mut fixture = complete_build_proof();
        for sample in &mut fixture.proof.samples {
            sample.build_caption = "the-one-and-only-caption".into();
        }
        let message = rejection(&fixture);
        assert!(message.contains("is carried by both"), "{message}");
        assert!(
            print_build_proof(&fixture.proof, &strictly(TranscriptSource::as_recorded())).is_err(),
            "one caption for every build must never print a ratio"
        );

        // Two samples is enough; it does not take fifteen.
        let mut fixture = complete_build_proof();
        let stolen = fixture.proof.samples[0].build_caption.clone();
        fixture.proof.samples[3].build_caption = stolen;
        assert!(rejection(&fixture).contains("is carried by both"));

        // A parent seed is a build too, and it may not borrow a sample's
        // caption or another seed's.
        let mut fixture = complete_build_proof();
        let seed_caption = fixture.proof.samples[2]
            .parent_seed
            .as_ref()
            .unwrap()
            .build_caption
            .clone();
        fixture.proof.samples[5]
            .parent_seed
            .as_mut()
            .unwrap()
            .build_caption = seed_caption;
        let message = rejection(&fixture);
        assert!(message.contains("parent seed"), "{message}");
        assert!(message.contains("is carried by both"), "{message}");

        let mut fixture = complete_build_proof();
        let sample_caption = fixture.proof.samples[2].build_caption.clone();
        fixture.proof.samples[2]
            .parent_seed
            .as_mut()
            .unwrap()
            .build_caption = sample_caption;
        assert!(rejection(&fixture).contains("is carried by both"));
    }

    #[test]
    fn the_empty_cache_hit_floor_is_the_verifier_s_and_defaults_to_strict() {
        // cargo invokes `rustc -vV` twice per build, and Incredibuild serves
        // the second invocation the entry the first stored. A genuine
        // empty-cache Rust build therefore reports one hit, and a validator
        // demanding zero refuses the truth -- which is a false refusal, not
        // rigour. The floor exists for that. It is passed by the person doing
        // the checking, never carried in the receipt, and it is not a way to
        // make a receipt pass: it raises the bar for warm samples by exactly
        // what it lowers for cold ones.
        let mut fixture = complete_build_proof();
        for sample in &mut fixture.proof.samples {
            if sample.mode == "ib-cold" {
                sample.cache_hits = Some(1);
            }
        }
        let message = rejection(&fixture);
        assert!(message.contains("was not empty-cache"), "{message}");
        assert!(message.contains("floor=0"), "{message}");

        let lenient = strictly(TranscriptSource::as_recorded()).with_empty_cache_hit_floor(1);
        assert!(
            validate_build_proof(&fixture.proof, &lenient).is_ok(),
            "a verifier who accepts the documented self-hit gets a summary"
        );

        // ... and pays for it on the other side: a warm sample whose hits do
        // not EXCEED the floor is no longer distinguishable from a cold one.
        warm_mut(&mut fixture.proof).cache_hits = Some(1);
        let error = validate_build_proof(&fixture.proof, &lenient)
            .expect_err("a warm sample at the floor is not warm");
        assert!(
            format!("{error:#}").contains("no verified cache hits above"),
            "{error:#}"
        );

        // The parent seed builds on the same emptied cache and hits the same
        // floor, and is judged by the same number.
        let mut fixture = complete_build_proof();
        warm_mut(&mut fixture.proof)
            .parent_seed
            .as_mut()
            .unwrap()
            .cache_hits = 1;
        assert!(rejection(&fixture).contains("did not run against an emptied cache"));
        assert!(validate_build_proof(&fixture.proof, &lenient).is_ok());

        // The floor is zero unless a human types otherwise.
        let cli = Cli::try_parse_from([
            "swf-cli",
            "robot-demo",
            "build-proof",
            "--receipt",
            "r.json",
        ])
        .unwrap();
        let Commands::RobotDemo {
            cmd:
                RobotDemoCommands::BuildProof {
                    empty_cache_hit_floor,
                    ..
                },
        } = cli.command
        else {
            panic!("expected build-proof command");
        };
        assert_eq!(empty_cache_hit_floor, 0);
    }

    #[test]
    fn the_checked_tier_names_no_document_and_the_not_checked_tier_names_them_all() {
        // The three paragraphs are the part of this tool a conference audience
        // actually reads, and they were the part still asserting things the
        // validator had never done -- a record-time Build History check,
        // printed under the word CHECKED, on a run that never opened a Build
        // History document. They exist as functions so that a test can read
        // them; as literals inside println! nothing could.
        let checked = checked_paragraph(&strictly(TranscriptSource::as_recorded()));
        for document in [
            "Build History",
            "cache-statistics",
            "Build Cache report",
            "reporting success",
        ] {
            assert!(
                !checked.contains(document),
                "the CHECKED tier must not name a document this invocation never opened: \
                 {document:?}"
            );
        }
        assert!(
            !checked.contains("independent sample"),
            "distinct repetition numbers are not independence, and the receipt's writer \
             picks them"
        );
        // What it DOES claim about ordering has to be true of warm samples
        // too, where the parent seed deliberately sits between the clear and
        // the measured build -- a build with nothing between it and a cache
        // clear could not have cache hits.
        assert!(
            checked.contains("between the clear and its PARENT SEED"),
            "{checked}"
        );
        assert!(checked.contains("the seed itself ran between"), "{checked}");

        let not_checked = not_checked_paragraph();
        for residue in [
            "Build History",
            "cache-statistics",
            "Build Cache report",
            "parent-seed counters",
            "wall_ms",
            "WHEN EACH SAMPLE WAS RECORDED",
            "signed",
        ] {
            assert!(
                not_checked.contains(residue),
                "the NOT CHECKED tier is where a reader finds what to distrust; \
                 {residue:?} is missing"
            );
        }

        // The RE-OBSERVED tier speaks only about files, and says so.
        let fixture = complete_build_proof();
        let observed = re_observed_paragraph(&accepted(&fixture).cache);
        assert!(
            observed.contains("10 cache-clear transcript file(s)"),
            "{observed}"
        );
        assert!(
            observed.contains("not the number of records in the receipt"),
            "{observed}"
        );
    }

    #[test]
    fn the_digest_is_computed_over_the_bytes_that_are_parsed() {
        // Hashing the path and then reading the path are two opens, and the
        // bytes hashed are then not provably the bytes parsed. Everything here
        // reads once and hashes the buffer it parsed. The digest is still
        // exactly what evidence::sha256_file produces, so a receipt recorded
        // by one is checkable by the other and the two cannot drift.
        let dir = TempDir::new("digest");
        let path = transcript_file(&dir, "user clear", 0, 1_000, 1_200);
        let (digest, text) = read_and_digest(&path).expect("a file this test just wrote");
        assert_eq!(digest, evidence::sha256_file(&path).unwrap());
        assert_eq!(text, std::fs::read_to_string(&path).unwrap());
        assert_eq!(digest.len(), 64);
        assert_eq!(load_cache_clear(&path).unwrap().transcript_sha256, digest);
    }

    #[test]
    fn the_cache_tool_a_transcript_names_is_reported_not_discarded() {
        // v2 exists so a transcript can say WHICH tool emptied the cache. The
        // parser read `tool=` and threw it away, so a transcript recording
        // `tool=/bin/true` validated silently. It now reaches the summary,
        // where a human can see what actually ran.
        let fixture = complete_build_proof();
        assert_eq!(
            accepted(&fixture).cache.tools,
            BTreeSet::from([TEST_CACHE_TOOL.to_string()])
        );

        let mut fixture = complete_build_proof();
        let real = fixture.proof.samples[1].cache_clears[0].clone();
        let path = fixture.dir.path().join("clear-bin-true.txt");
        std::fs::write(
            &path,
            transcript_text(
                "/bin/true",
                "user clear",
                0,
                real.started_at_ms,
                real.completed_at_ms,
            ),
        )
        .unwrap();
        fixture.proof.samples[1].cache_clears = vec![load_cache_clear(&path).unwrap()];
        let tools = accepted(&fixture).cache.tools;
        assert!(
            tools.contains("/bin/true"),
            "a transcript naming /bin/true must surface it: {tools:?}"
        );
    }

    #[test]
    fn v2_transcripts_must_name_a_tool_and_a_command_that_matches_their_argv() {
        let ok = transcript_text(TEST_CACHE_TOOL, "user clear", 0, 1_000, 1_200);
        let parsed = parse_clear_transcript(&ok).unwrap();
        assert_eq!(parsed.marker_version, 2);
        assert_eq!(parsed.tool.as_deref(), Some(TEST_CACHE_TOOL));
        assert_eq!(parsed.scope, CacheScope::LocalUser);

        // A command line that is not the tool followed by the argv means the
        // three lines cannot all be describing the same invocation.
        let mismatched = ok.replace(
            &format!("command={TEST_CACHE_TOOL} user clear"),
            &format!("command={TEST_CACHE_TOOL} shared clear"),
        );
        let message = parse_clear_transcript(&mismatched).unwrap_err().to_string();
        assert!(message.contains("internally inconsistent"), "{message}");

        for missing in ["tool", "command"] {
            let stripped = ok
                .lines()
                .filter(|line| !line.starts_with(&format!("{missing}=")))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                parse_clear_transcript(&stripped).is_err(),
                "a v2 transcript without {missing} must not parse"
            );
        }
    }

    #[test]
    fn a_v1_transcript_corroborates_the_timeline_but_never_claims_which_tool_ran() {
        // v1's recorder wrote `argv=$*` after the tool name had been shifted
        // off, so the file cannot say what acted on the cache. Everything it
        // CAN say is still cross-checked; the one thing it cannot establish
        // is never claimed -- it is counted separately, and print_build_proof
        // reports that count on its own line.
        let mut fixture = complete_build_proof();
        let real = fixture.proof.samples[1].cache_clears[0].clone();
        let path = fixture.dir.path().join("clear-v1.txt");
        std::fs::write(
            &path,
            format!(
                "{}\nargv=user clear\nexit_code=0\nstarted_at_ms={}\ncompleted_at_ms={}\n\
                 --- transcript ---\nLocal user build cache cleared.\n",
                CACHE_CLEAR_MARKERS[0], real.started_at_ms, real.completed_at_ms
            ),
        )
        .unwrap();
        fixture.proof.samples[1].cache_clears =
            vec![load_cache_clear(&path).expect("v1 transcripts still parse")];

        let summary = accepted(&fixture);
        assert_eq!(summary.cache.transcripts, 10);
        assert_eq!(
            summary.cache.anonymous_transcripts, 1,
            "a v1 transcript must be counted as one that cannot name its tool"
        );
        assert!(
            !summary.cache.tools.contains(""),
            "a nameless tool is never recorded as a tool"
        );

        // And the timeline it does record is corroborated like any other.
        fixture.proof.samples[1].cache_clears[0].started_at_ms += 1;
        let message = rejection(&fixture);
        assert!(message.contains("disagrees with transcript"), "{message}");
        assert!(message.contains("started_at_ms"), "{message}");
    }

    #[test]
    fn a_v1_transcript_naming_a_tool_could_not_have_been_written_by_the_v1_recorder() {
        let edited = format!(
            "{}\ntool={TEST_CACHE_TOOL}\nargv=user clear\nexit_code=0\nstarted_at_ms=1000\n\
             completed_at_ms=1200\n--- transcript ---\nLocal user build cache cleared.\n",
            CACHE_CLEAR_MARKERS[0]
        );
        let message = parse_clear_transcript(&edited).unwrap_err().to_string();
        assert!(message.contains("could not have written"), "{message}");
    }

    #[test]
    fn build_proof_defaults_to_the_recorded_paths_and_takes_a_relocation_directory() {
        fn parsed(args: &[&str]) -> (Option<PathBuf>, usize) {
            let cli = Cli::try_parse_from(args).expect("valid build-proof invocation");
            let Commands::RobotDemo {
                cmd:
                    RobotDemoCommands::BuildProof {
                        transcripts,
                        min_samples,
                        ..
                    },
            } = cli.command
            else {
                panic!("expected build-proof command");
            };
            (transcripts, min_samples)
        }

        // Omitting --transcripts is not a way to skip the check. It means the
        // paths the receipt recorded must resolve as recorded.
        assert_eq!(
            parsed(&[
                "swf-cli",
                "robot-demo",
                "build-proof",
                "--receipt",
                "r.json"
            ]),
            (None, 5)
        );
        assert_eq!(
            parsed(&[
                "swf-cli",
                "robot-demo",
                "build-proof",
                "--receipt",
                "r.json",
                "--transcripts",
                "evidence/raw",
            ]),
            (Some(PathBuf::from("evidence/raw")), 5)
        );
        assert!(Cli::try_parse_from([
            "swf-cli",
            "robot-demo",
            "build-proof",
            "--receipt",
            "r.json",
            "--min-samples",
            "0",
        ])
        .is_err());
    }

    #[test]
    fn transcripts_relocate_by_basename_and_relocation_is_never_a_bypass() {
        let fixture = complete_build_proof();
        let elsewhere = TempDir::new("relocated");
        for entry in std::fs::read_dir(fixture.dir.path()).unwrap() {
            let entry = entry.unwrap();
            std::fs::copy(entry.path(), elsewhere.path().join(entry.file_name())).unwrap();
        }

        // The same receipt, checked on the host that produced it and beside a
        // copy of its evidence. Both open and re-hash every file; only the
        // search moved. ib-benchmark.sh records host-absolute paths, so
        // without this a receipt is only checkable on one machine.
        assert!(
            validate_build_proof(&fixture.proof, &strictly(TranscriptSource::as_recorded()))
                .is_ok()
        );
        assert!(validate_build_proof(
            &fixture.proof,
            &strictly(TranscriptSource::relocated(elsewhere.path().into()))
        )
        .is_ok());

        // A reviewer who kept the receipt and none of the transcripts gets a
        // refusal, not a summary.
        let empty = TempDir::new("empty");
        let error = validate_build_proof(
            &fixture.proof,
            &strictly(TranscriptSource::relocated(empty.path().into())),
        )
        .expect_err("relocation must not waive the check");
        assert!(format!("{error:#}").contains("does not waive it"));

        // And a tampered copy inside the relocation directory is refused
        // there too: --transcripts moves the search, it does not lower the
        // bar once the file is found.
        let name = Path::new(&fixture.proof.samples[1].cache_clears[0].transcript_path)
            .file_name()
            .unwrap()
            .to_owned();
        let moved = elsewhere.path().join(&name);
        let text = std::fs::read_to_string(&moved).unwrap();
        std::fs::write(
            &moved,
            text.replace(
                "Local user build cache cleared.",
                "Shared cache cleared. (and this line was typed by hand)",
            ),
        )
        .unwrap();
        let error = validate_build_proof(
            &fixture.proof,
            &strictly(TranscriptSource::relocated(elsewhere.path().into())),
        )
        .expect_err("a tampered relocated transcript must be refused");
        assert!(format!("{error:#}").contains("hashes to"));
    }

    #[test]
    fn two_recorded_paths_cannot_relocate_onto_the_same_transcript() {
        // Under --transcripts only the basename survives, so two different
        // recorded paths sharing one can no longer be told apart. Which file
        // a clear refers to is then unestablished, and unestablished is a
        // refusal rather than a coin flip.
        let mut fixture = complete_build_proof();
        let flat = TempDir::new("flat");
        for entry in std::fs::read_dir(fixture.dir.path()).unwrap() {
            let entry = entry.unwrap();
            std::fs::copy(entry.path(), flat.path().join(entry.file_name())).unwrap();
        }

        let real = fixture.proof.samples[1].cache_clears[0].clone();
        let base = Path::new(&real.transcript_path)
            .file_name()
            .unwrap()
            .to_owned();
        let nested = fixture.dir.path().join("other");
        std::fs::create_dir_all(&nested).unwrap();
        let twin = nested.join(&base);
        std::fs::copy(&real.transcript_path, &twin).unwrap();

        // A byte-identical file at a different recorded path: same digest,
        // same contents, and still ambiguous once the directory is flattened.
        let mut second = real.clone();
        second.transcript_path = twin.display().to_string();
        warm_mut(&mut fixture.proof).cache_clears = vec![second];

        let error = validate_build_proof(
            &fixture.proof,
            &strictly(TranscriptSource::relocated(flat.path().into())),
        )
        .expect_err("an ambiguous relocation must be refused");
        let message = format!("{error:#}");
        assert!(message.contains("both relocate to"), "{message}");
        assert!(message.contains("cannot be established"), "{message}");
    }

    #[test]
    fn a_recorded_path_cannot_escape_the_relocation_directory() {
        // Relocation honours only `Path::file_name`, which never yields "."
        // or "..". Without that, a receipt naming "../somewhere/mine.txt"
        // would pick its own evidence from outside the directory the operator
        // pointed at, and --transcripts would become a way to be handed a
        // file the receipt chose.
        let mut fixture = complete_build_proof();
        let real = PathBuf::from(&fixture.proof.samples[1].cache_clears[0].transcript_path);
        let outside = TempDir::new("outside");
        std::fs::copy(&real, outside.path().join("escape.txt")).unwrap();
        fixture.proof.samples[1].cache_clears[0].transcript_path = format!(
            "../{}/escape.txt",
            outside.path().file_name().unwrap().to_string_lossy()
        );
        let dir: PathBuf = fixture.dir.path().into();
        let error =
            validate_build_proof(&fixture.proof, &strictly(TranscriptSource::relocated(dir)))
                .expect_err("only the basename may be honoured");
        let message = format!("{error:#}");
        assert!(message.contains("is not in"), "{message}");
        assert!(message.contains("escape.txt"), "{message}");
    }

    /// Sibling of `cache_state_is_never_asserted_by_construction`, guarding
    /// the hole that replaced the one that test guards. The first fix moved
    /// the defect instead of closing it: schema v2 added transcripts, and the
    /// validator then checked that the digest LOOKED like a digest. The way
    /// that comes back is a way to not open the file.
    #[test]
    fn corroboration_has_no_opt_out() {
        let source = include_str!("main.rs");

        // Assembled at runtime so this guard is never itself an occurrence.
        for banned in [
            format!("{}_transcripts", "skip"),
            format!("{}_corroboration", "skip"),
            format!("{}_transcripts", "assume"),
            format!("{}_digest", "trust"),
            format!("{}_valid", "assume"),
        ] {
            assert!(
                !source.contains(&banned),
                "{banned} is back: a way to not look at the evidence is a way to pass a \
                 false receipt, and it would be the path most of these tests ran through"
            );
        }

        // Exactly two ways to name a transcript source, and neither of them
        // means "do not look".
        for constructor in ["as_recorded", "relocated"] {
            assert_eq!(
                source.matches(&format!("fn {constructor}(")).count(),
                1,
                "{constructor} must remain the only constructor of its kind"
            );
        }

        // The digest is recomputed from the file that was opened, and then
        // compared against the field the receipt asserts.
        // The digest is recomputed from the bytes that were read, in the
        // same read, and then compared against the field the receipt asserts.
        assert!(source.contains(&format!("{}(&resolved)", "read_and_digest")));
        assert!(source.contains(&format!("found.digest != clear.{}", "transcript_sha256")));

        // And the counts the summary prints come from the ledger of files
        // opened, never from a counter walked over the receipt's own records.
        assert!(source.contains(&format!("transcripts: ledger.read.{}()", "len")));
    }
}
