//! Exercise the real subprocess/session boundary with tiny protocol peers.
use robot_safety_gate::Policy;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use swf_app::evidence::{append_scenario_result, ScenarioResult};
use swf_app::{Session, SessionConfig};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "swf-session-test-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn session(&self, script: &str, timeout: Duration) -> Session {
        let bridge = self.0.join("peer.py");
        std::fs::write(&bridge, format!("{PRELUDE}\n{script}\n")).unwrap();
        Session::spawn(SessionConfig {
            run_id: "run".into(),
            scenario_name: "case".into(),
            scenario_path: self.0.join("unused.json"),
            bridge_path: bridge,
            python: std::env::var("ROBOT_DEMO_PYTHON").unwrap_or_else(|_| "python3".into()),
            backend: "mock".into(),
            evidence_dir: self.0.clone(),
            proposal_timeout: timeout,
            policy: Policy::default(),
        })
        .unwrap()
    }

    fn run(&self, script: &str) -> ScenarioResult {
        self.session(script, Duration::from_secs(3)).run().unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const PRELUDE: &str = r#"
import json, sys, time
def emit(message):
    print(json.dumps(message), flush=True)
hello = dict(type='hello', backend='mock', version=1, simulation_tick_ns=50_000_000)
proposal = dict(type='proposal', run_id='run', episode_id='run-case', action_id='a',
    simulation_tick=0, simulation_time_ns=0,
    observation=dict(observation_id='obs-0', capture_time_ns=0), simulated_stop=False,
    proposed_action=dict(kind='pickup', target='cube', segment='approach'))
outcome = dict(type='outcome', action_id='a', simulation_tick=0, dispatched=True, action_kind='task')
end = dict(type='episode_end', success=True, reason='cube_lifted', ticks=1)
def propose():
    emit(proposal)
    decision = json.loads(sys.stdin.readline())
    assert decision['run_id'] == 'run'
    assert decision['episode_id'] == 'run-case'
    assert decision['action_id'] == proposal['action_id']
    assert decision['simulation_tick'] == proposal['simulation_tick']
    return decision
"#;

#[test]
fn counts_confirmed_dispatches_and_echoes_identity() {
    let result = Fixture::new().run("emit(hello)\npropose()\nemit(outcome)\nemit(end)");
    assert_eq!(result.outcome, "cube_lifted");
    assert!(result.success);
    assert_eq!(result.decisions, 1);
    assert_eq!(result.task_dispatches, 1);
}

#[test]
fn a_permit_without_an_outcome_is_not_a_dispatch() {
    let result = Fixture::new().run("emit(hello)\npropose()");
    assert_eq!(result.outcome, "bridge_died");
    assert_eq!(result.decisions, 1);
    assert_eq!(result.task_dispatches, 0);
    assert!(!result.success);
}

#[test]
fn rejection_is_recorded_without_dispatch() {
    let result = Fixture::new().run(
        "emit(hello)\nproposal['simulated_stop'] = True\nassert propose()['decision'] == 'reject'\n\
         outcome['dispatched'] = False\nemit(outcome)\n\
         emit(dict(type='hold', simulation_tick=0, simulation_time_ns=0, reason='rejection:emergency_stop'))\n\
         end.update(success=False, reason='emergency_stop', ticks=0)\nemit(end)",
    );
    assert_eq!(result.outcome, "emergency_stop");
    assert_eq!(result.task_dispatches, 0);
    assert_eq!(result.rejections, ["reject/emergency_stop"]);
}

#[test]
fn invalid_protocol_sequences_fail_closed() {
    let scripts = [
        ("proposal before hello", "emit(proposal)"),
        ("duplicate hello", "emit(hello)\nemit(hello)"),
        ("wrong version", "hello['version'] = 2\nemit(hello)"),
        ("wrong backend", "hello['backend'] = 'robosuite'\nemit(hello)"),
        ("wrong clock", "hello['simulation_tick_ns'] = 0\nemit(hello)"),
        ("wrong run", "emit(hello)\nproposal['run_id'] = 'other'\nemit(proposal)"),
        ("wrong episode", "emit(hello)\nproposal['episode_id'] = 'other'\nemit(proposal)"),
        ("empty action", "emit(hello)\nproposal['action_id'] = ''\nemit(proposal)"),
        ("inconsistent time", "emit(hello)\nproposal['simulation_time_ns'] = 1\nemit(proposal)"),
        ("two pending proposals", "emit(hello)\npropose()\nemit(proposal)"),
        ("outcome before proposal", "emit(hello)\nemit(outcome)"),
        ("wrong outcome action", "emit(hello)\npropose()\noutcome['action_id'] = 'b'\nemit(outcome)"),
        ("wrong outcome tick", "emit(hello)\npropose()\noutcome['simulation_tick'] = 1\nemit(outcome)"),
        ("wrong outcome kind", "emit(hello)\npropose()\noutcome['action_kind'] = 'hold'\nemit(outcome)"),
        ("dispatch after reject", "emit(hello)\nproposal['simulated_stop'] = True\npropose()\nemit(outcome)"),
        ("end before outcome", "emit(hello)\npropose()\nemit(end)"),
        ("success without dispatch", "emit(hello)\nemit(end)"),
        ("hold before outcome", "emit(hello)\npropose()\nemit(dict(type='hold', simulation_tick=1, simulation_time_ns=50_000_000, reason='hold'))"),
    ];
    for (label, script) in scripts {
        let result = Fixture::new().run(script);
        assert_eq!(result.outcome, "protocol_error", "{label}");
        assert!(!result.success, "{label}");
        assert_eq!(result.task_dispatches, 0, "{label}");
    }
}

#[test]
fn repeated_actions_outcomes_and_regressing_ticks_are_rejected() {
    let scripts = [
        "emit(outcome)",
        "proposal.update(simulation_tick=1, simulation_time_ns=50_000_000)\nemit(proposal)",
        "proposal['action_id'] = 'b'\nemit(proposal)",
        "end['ticks'] = 0\nemit(end)",
        "emit(dict(type='hold', simulation_tick=2, simulation_time_ns=100_000_000, reason='hold'))\n\
         emit(dict(type='hold', simulation_tick=1, simulation_time_ns=50_000_000, reason='hold'))",
    ];
    for script in scripts {
        let result =
            Fixture::new().run(&format!("emit(hello)\npropose()\nemit(outcome)\n{script}"));
        assert_eq!(result.outcome, "protocol_error", "{script}");
        assert_eq!(result.task_dispatches, 1);
    }
}

#[test]
fn malformed_messages_are_logged_as_valid_json() {
    let fixture = Fixture::new();
    let result = fixture.run("emit(dict(type='unknown\"message'))");
    assert_eq!(result.outcome, "protocol_error");
    let trace = std::fs::read_to_string(fixture.0.join("events-case.jsonl")).unwrap();
    let last: serde_json::Value = serde_json::from_str(trace.lines().last().unwrap()).unwrap();
    let event: serde_json::Value = serde_json::from_str(last["line"].as_str().unwrap()).unwrap();
    assert_eq!(event["session"], "malformed_message");
    assert!(event["error"].as_str().unwrap().contains("unknown"));
}

#[test]
fn timeouts_reap_the_bridge_and_never_imply_dispatch() {
    let fixture = Fixture::new();
    let result = fixture
        .session("emit(hello)\ntime.sleep(10)", Duration::from_millis(300))
        .run()
        .unwrap();
    assert_eq!(result.outcome, "timeout");
    assert_eq!(result.task_dispatches, 0);
    assert!(result.wall_time_ms < 3_000);
}

#[test]
fn unwritable_event_log_returns_an_error() {
    let fixture = Fixture::new();
    std::fs::create_dir(fixture.0.join("events-case.jsonl")).unwrap();
    assert!(fixture
        .session("time.sleep(10)", Duration::from_secs(3))
        .run()
        .is_err());
}

#[test]
fn corrupt_results_are_preserved_instead_of_silently_overwritten() {
    let fixture = Fixture::new();
    let result = fixture.run("emit(hello)\npropose()\nemit(outcome)\nemit(end)");
    let path = fixture.0.join("scenario-results.json");
    std::fs::write(&path, "broken evidence").unwrap();
    assert!(append_scenario_result(&fixture.0, &result).is_err());
    assert_eq!(std::fs::read_to_string(path).unwrap(), "broken evidence");
}
