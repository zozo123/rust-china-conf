//! PROTECTED ACCEPTANCE FIXTURE — DO NOT EDIT.
//!
//! This suite is the protected contract for the robot-safety-gate decision
//! function. The agent work order for the seeded-regression exercise
//! explicitly forbids modifying this file; the factory's diff validator
//! rejects candidate patches that touch it.
//!
//! Matrix (from the demo plan, "Acceptance criteria"):
//!   age 0 / 50 / 250 ms, no stop   -> Permit
//!   age 251 / 600 ms, no stop      -> Reject(StalePerception)
//!   stop with fresh data           -> Reject(EmergencyStop)
//!   stop with stale data           -> Reject(EmergencyStop)
//!   future/invalid timestamp       -> Reject(InvalidTimestamp)
//!   boundary 250 / 251 ms          -> exact behavior pinned

use robot_safety_gate::{
    decide, Decision, ObservationRef, Policy, Proposal, ProposedAction, RejectionReason,
    NS_PER_MS,
};

const NOW_NS: u64 = 1_000_000_000;

fn proposal(age_ms: u64, stop: bool) -> Proposal {
    Proposal {
        run_id: "contract".into(),
        episode_id: "contract-ep".into(),
        action_id: format!("a-{age_ms}-{stop}"),
        simulation_tick: 20,
        simulation_time_ns: NOW_NS,
        observation: ObservationRef {
            observation_id: "obs".into(),
            capture_time_ns: NOW_NS - age_ms * NS_PER_MS,
        },
        simulated_stop: stop,
        proposed_action: ProposedAction::Pickup {
            target: "cube".into(),
            segment: "approach".into(),
        },
    }
}

fn permit() -> Decision {
    Decision::Permit
}

fn reject(reason: RejectionReason) -> Decision {
    Decision::Reject { reason }
}

#[test]
fn fresh_ages_are_permitted() {
    let policy = Policy::default();
    for age_ms in [0_u64, 50, 250] {
        assert_eq!(
            decide(&proposal(age_ms, false), &policy),
            permit(),
            "age {age_ms} ms must be permitted"
        );
    }
}

#[test]
fn stale_ages_are_rejected() {
    let policy = Policy::default();
    for age_ms in [251_u64, 600] {
        assert_eq!(
            decide(&proposal(age_ms, false), &policy),
            reject(RejectionReason::StalePerception { age_ms }),
            "age {age_ms} ms must be rejected as stale"
        );
    }
}

#[test]
fn boundary_250ms_permits() {
    assert_eq!(
        decide(&proposal(250, false), &Policy::default()),
        permit()
    );
}

#[test]
fn boundary_251ms_rejects() {
    assert_eq!(
        decide(&proposal(251, false), &Policy::default()),
        reject(RejectionReason::StalePerception { age_ms: 251 })
    );
}

#[test]
fn stop_with_fresh_data_rejects_emergency_stop() {
    assert_eq!(
        decide(&proposal(0, true), &Policy::default()),
        reject(RejectionReason::EmergencyStop)
    );
}

#[test]
fn stop_with_stale_data_rejects_emergency_stop() {
    // Stop precedence: even a stale observation must report EmergencyStop,
    // never StalePerception.
    assert_eq!(
        decide(&proposal(600, true), &Policy::default()),
        reject(RejectionReason::EmergencyStop)
    );
}

#[test]
fn future_timestamp_rejected() {
    let mut p = proposal(0, false);
    p.observation.capture_time_ns = p.simulation_time_ns + 1; // 1 ns in the future
    assert_eq!(
        decide(&p, &Policy::default()),
        reject(RejectionReason::InvalidTimestamp)
    );
}

#[test]
fn configured_threshold_is_respected() {
    let tight = Policy {
        max_observation_age_ms: 100,
    };
    assert_eq!(
        decide(&proposal(150, false), &tight),
        reject(RejectionReason::StalePerception { age_ms: 150 })
    );
    assert_eq!(decide(&proposal(100, false), &tight), permit());
}
