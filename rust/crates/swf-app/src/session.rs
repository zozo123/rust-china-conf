//! Lock-step session between the Rust gate and the Python simulator bridge.
//!
//! Invariants enforced here (see the demo plan, "Timing and action protocol"):
//!   * one task action outstanding at a time (the bridge awaits a decision)
//!   * a missing, malformed or timed-out bridge message fails the episode
//!     explicitly — never a default authorization
//!   * approvals echo the exact action id and simulation tick; the bridge
//!     must refuse old, duplicate or mismatched approvals
//!   * rejections produce no task-action dispatch; hold steps are labeled
//!
//! Wall-clock timeouts govern the *session*; the task contract uses the
//! simulation clock carried inside proposals. The two never mix.

use crate::evidence::{EventLog, ScenarioResult};
use crate::protocol::{parse_bridge_line, BridgeMsg, DecisionMsg};
use robot_safety_gate::{decide, Decision, Policy};
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub run_id: String,
    pub scenario_name: String,
    pub scenario_path: PathBuf,
    pub bridge_path: PathBuf,
    pub python: String,
    pub backend: String,
    pub evidence_dir: PathBuf,
    pub proposal_timeout: Duration,
    pub policy: Policy,
}

pub struct Session {
    cfg: SessionConfig,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    decisions: u32,
    rejections: Vec<String>,
    backend: String,
}

/// Outcome categories kept visible in evidence:
///   cube_lifted | task_incomplete | rejected_stale | rejected_invalid
///   | emergency_stop | timeout | protocol_error | bridge_died
impl Session {
    pub fn spawn(cfg: SessionConfig) -> std::io::Result<Self> {
        std::fs::create_dir_all(&cfg.evidence_dir)?;
        let stderr_log = cfg.evidence_dir.join("bridge-stderr.log");
        let stderr_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(stderr_log)?;

        let mut command = Command::new(&cfg.python);
        command
            .arg(&cfg.bridge_path)
            .arg("--scenario")
            .arg(&cfg.scenario_path)
            .arg("--backend")
            .arg(&cfg.backend)
            .arg("--run-id")
            .arg(&cfg.run_id)
            .arg("--episode-id")
            .arg(format!("{}-{}", cfg.run_id, cfg.scenario_name));
        if let Ok(video_dir) = std::env::var("ROBOT_DEMO_VIDEO_DIR") {
            command.arg("--video-dir").arg(video_dir);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(stderr_file))
            .spawn()?;

        let stdin = child.stdin.take();
        Ok(Self {
            cfg,
            child: Some(child),
            stdin,
            decisions: 0,
            rejections: Vec::new(),
            backend: "unknown".into(),
        })
    }

    pub fn run(mut self) -> std::io::Result<ScenarioResult> {
        let started = Instant::now();
        let mut log = EventLog::create(&self.cfg.evidence_dir, &self.cfg.scenario_name)?;
        let mut protocol = ProtocolState::new(&self.cfg);

        let stdout = self
            .child
            .as_mut()
            .and_then(|c| c.stdout.take())
            .expect("child stdout piped");

        // Reader thread: one mpsc message per stdout line.
        let (tx, rx) = mpsc::sync_channel::<String>(1);
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if tx.send(l).is_err() {
                            return;
                        }
                    }
                    Err(_) => return,
                }
            }
        });

        let mut outcome = "bridge_died".to_string();
        let mut success = false;
        let mut ticks = 0_u64;

        loop {
            let line = match rx.recv_timeout(self.cfg.proposal_timeout) {
                Ok(l) => l,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    log.record("session", "{\"session\":\"proposal_timeout\"}")?;
                    outcome = "timeout".into();
                    break;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break, // bridge exited
            };

            log.record("in", &line)?;

            let msg = match parse_bridge_line(&line) {
                Ok(m) => m,
                Err(e) => {
                    log.record(
                        "session",
                        &serde_json::json!({"session": "malformed_message", "error": e.to_string()}).to_string(),
                    )?;
                    outcome = "protocol_error".into();
                    break;
                }
            };

            let decision = match protocol.observe(&msg, &self.cfg.policy) {
                Ok(decision) => decision,
                Err(error) => {
                    log.record(
                        "session",
                        &serde_json::json!({"session": "protocol_error", "error": error})
                            .to_string(),
                    )?;
                    outcome = "protocol_error".into();
                    break;
                }
            };

            match msg {
                BridgeMsg::Hello(h) => {
                    self.backend = if h.backend_label.is_empty() {
                        h.backend
                    } else {
                        h.backend_label
                    };
                }
                BridgeMsg::Proposal(p) => {
                    self.decisions += 1;
                    let d = decision.expect("validated proposal has a decision");
                    if !d.is_permit() {
                        self.rejections.push(d.to_string());
                    }
                    let msg = DecisionMsg::for_proposal(&p, d);
                    let wire = serde_json::to_string(&msg).expect("decision serializes");
                    log.record("out", &wire)?;
                    if let Some(stdin) = self.stdin.as_mut() {
                        if writeln!(stdin, "{wire}")
                            .and_then(|_| stdin.flush())
                            .is_err()
                        {
                            outcome = "bridge_died".into();
                            break;
                        }
                    }
                }
                BridgeMsg::Outcome(_) => {}
                BridgeMsg::Hold(h) => {
                    ticks = ticks.max(h.simulation_tick);
                }
                BridgeMsg::EpisodeEnd(e) => {
                    outcome = e.reason.clone();
                    success = e.success;
                    ticks = ticks.max(e.ticks);
                    break;
                }
            }
        }

        self.terminate();
        Ok(ScenarioResult {
            scenario: self.cfg.scenario_name.clone(),
            backend: self.backend.clone(),
            outcome,
            success,
            task_dispatches: protocol.task_dispatches,
            decisions: self.decisions,
            rejections: self.rejections.clone(),
            ticks,
            wall_time_ms: started.elapsed().as_millis(),
        })
    }

    fn terminate(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Also reap the bridge on evidence I/O errors or unwinding.
        self.terminate();
    }
}

struct PendingAction {
    action_id: String,
    tick: u64,
    permitted: bool,
}

/// Keep protocol integrity separate from the demonstration's gate policy.
/// A permit is an authorization; only a matching outcome confirms dispatch.
struct ProtocolState {
    run_id: String,
    episode_id: String,
    backend: String,
    tick_ns: Option<u64>,
    last_tick: u64,
    pending: Option<PendingAction>,
    seen_actions: HashSet<String>,
    last_dispatch_tick: Option<u64>,
    rejected: bool,
    task_dispatches: u32,
}

impl ProtocolState {
    fn new(cfg: &SessionConfig) -> Self {
        Self {
            run_id: cfg.run_id.clone(),
            episode_id: format!("{}-{}", cfg.run_id, cfg.scenario_name),
            backend: cfg.backend.clone(),
            tick_ns: None,
            last_tick: 0,
            pending: None,
            seen_actions: HashSet::new(),
            last_dispatch_tick: None,
            rejected: false,
            task_dispatches: 0,
        }
    }

    fn check_clock(&self, tick: u64, time_ns: u64) -> Result<(), String> {
        if tick < self.last_tick {
            return Err("simulation tick moved backwards".into());
        }
        if self.tick_ns.and_then(|ns| tick.checked_mul(ns)) != Some(time_ns) {
            return Err("simulation time does not match tick".into());
        }
        Ok(())
    }

    fn observe(&mut self, msg: &BridgeMsg, policy: &Policy) -> Result<Option<Decision>, String> {
        if self.tick_ns.is_none() && !matches!(msg, BridgeMsg::Hello(_)) {
            return Err("first bridge message must be hello".into());
        }
        match msg {
            BridgeMsg::Hello(h) => {
                if self.tick_ns.is_some() {
                    return Err("duplicate hello".into());
                }
                if h.version != 1 || h.simulation_tick_ns != 50_000_000 || h.backend != self.backend
                {
                    return Err("unsupported protocol version, clock, or backend".into());
                }
                self.tick_ns = Some(h.simulation_tick_ns);
            }
            BridgeMsg::Proposal(p) => {
                if self.pending.is_some() {
                    return Err("proposal received before previous outcome".into());
                }
                if self.rejected {
                    return Err("proposal received after rejection".into());
                }
                if p.run_id != self.run_id || p.episode_id != self.episode_id {
                    return Err("proposal run or episode identity mismatch".into());
                }
                if p.action_id.is_empty() || self.seen_actions.contains(&p.action_id) {
                    return Err("empty or duplicate action identity".into());
                }
                self.check_clock(p.simulation_tick, p.simulation_time_ns)?;
                if self
                    .last_dispatch_tick
                    .is_some_and(|tick| p.simulation_tick <= tick)
                {
                    return Err("proposal tick did not advance after dispatch".into());
                }
                let decision = decide(p, policy);
                self.pending = Some(PendingAction {
                    action_id: p.action_id.clone(),
                    tick: p.simulation_tick,
                    permitted: decision.is_permit(),
                });
                self.last_tick = p.simulation_tick;
                self.rejected = !decision.is_permit();
                self.seen_actions.insert(p.action_id.clone());
                return Ok(Some(decision));
            }
            BridgeMsg::Outcome(o) => {
                let pending = self
                    .pending
                    .as_ref()
                    .ok_or("outcome without outstanding proposal")?;
                if o.action_id != pending.action_id
                    || o.simulation_tick != pending.tick
                    || o.action_kind != "task"
                {
                    return Err("outcome action, tick, or kind does not match proposal".into());
                }
                if o.dispatched && !pending.permitted {
                    return Err("task dispatched without permission".into());
                }
                if o.dispatched {
                    self.task_dispatches += 1;
                    self.last_dispatch_tick = Some(o.simulation_tick);
                }
                self.pending = None;
            }
            BridgeMsg::Hold(h) => {
                if self.pending.is_some() {
                    return Err("hold received before outstanding outcome".into());
                }
                self.check_clock(h.simulation_tick, h.simulation_time_ns)?;
                self.last_tick = h.simulation_tick;
            }
            BridgeMsg::EpisodeEnd(e) => {
                if self.pending.is_some() {
                    return Err("episode ended without outstanding outcome".into());
                }
                if e.ticks < self.last_tick
                    || self.last_dispatch_tick.is_some_and(|tick| e.ticks <= tick)
                {
                    return Err("episode ended with an inconsistent tick".into());
                }
                if e.success && (self.task_dispatches == 0 || self.rejected) {
                    return Err("episode reports success without permitted task dispatches".into());
                }
            }
        }
        Ok(None)
    }
}
