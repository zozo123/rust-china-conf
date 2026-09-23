//! robot-safety-gate: pure decision contract for the conference robot demo.
//!
//! The gate sits between a scripted controller (Python bridge) and the
//! simulator. Every guarded task action is proposed with the observation it
//! was computed from; the gate permits or rejects dispatch. Hold steps are
//! explicitly unguarded and never pass through here.
//!
//! The contract: observations older than the configured threshold (250 ms
//! by default) are rejected with `StalePerception` at the final dispatch
//! decision. `tests/contract.rs` encodes the required behavior.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Demo policy threshold. 250 ms is an illustrative demo value, NOT an
/// established safe threshold for physical robots.
pub const DEFAULT_MAX_OBSERVATION_AGE_MS: u64 = 250;
pub const NS_PER_MS: u64 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    pub max_observation_age_ms: u64,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            max_observation_age_ms: DEFAULT_MAX_OBSERVATION_AGE_MS,
        }
    }
}

/// Reference to the observation a proposal was computed from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationRef {
    pub observation_id: String,
    pub capture_time_ns: u64,
}

/// A guarded task action. Only task actions reach the gate; hold steps are
/// taken unilaterally by the bridge and are labeled separately in the trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProposedAction {
    Pickup { target: String, segment: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    pub run_id: String,
    pub episode_id: String,
    pub action_id: String,
    pub simulation_tick: u64,
    pub simulation_time_ns: u64,
    pub observation: ObservationRef,
    pub simulated_stop: bool,
    pub proposed_action: ProposedAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum RejectionReason {
    EmergencyStop,
    InvalidTimestamp,
    StalePerception { age_ms: u64 },
}

impl fmt::Display for RejectionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RejectionReason::EmergencyStop => write!(f, "emergency_stop"),
            RejectionReason::InvalidTimestamp => write!(f, "invalid_timestamp"),
            RejectionReason::StalePerception { age_ms } => {
                write!(f, "stale_perception(age={age_ms}ms)")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum Decision {
    Permit,
    Reject {
        #[serde(flatten)]
        reason: RejectionReason,
    },
}

impl Decision {
    pub fn is_permit(&self) -> bool {
        matches!(self, Decision::Permit)
    }
}

impl fmt::Display for Decision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Decision::Permit => write!(f, "permit"),
            Decision::Reject { reason } => write!(f, "reject/{reason}"),
        }
    }
}

/// Evaluate a task-action proposal at the final dispatch decision point.
///
/// Evaluation order is contractual:
///   1. simulated emergency stop — precedes every other check
///   2. timestamp validity — future/invalid observation timestamps reject
///   3. freshness — observation age vs. policy threshold (SEE SEEDED NOTE)
pub fn decide(proposal: &Proposal, policy: &Policy) -> Decision {
    // 1. Simulated emergency stop takes precedence over everything,
    //    including malformed timestamps.
    if proposal.simulated_stop {
        return Decision::Reject {
            reason: RejectionReason::EmergencyStop,
        };
    }

    // 2. Timestamp validation: a capture time after the current simulation
    //    time is invalid and must never authorize motion.
    if proposal.observation.capture_time_ns > proposal.simulation_time_ns {
        return Decision::Reject {
            reason: RejectionReason::InvalidTimestamp,
        };
    }

    let age_ms =
        (proposal.simulation_time_ns - proposal.observation.capture_time_ns) / NS_PER_MS;

    // 3. Freshness at the final dispatch decision: observations older than
    //    the configured threshold are rejected as stale. Boundary: exactly
    //    `max_observation_age_ms` permits; one millisecond beyond rejects.
    if age_ms > policy.max_observation_age_ms {
        return Decision::Reject {
            reason: RejectionReason::StalePerception { age_ms },
        };
    }
    Decision::Permit
}

#[cfg(test)]
mod unit {
    use super::*;

    fn proposal_at_age(age_ms: u64) -> Proposal {
        let now = 1_000_000_000_u64;
        Proposal {
            run_id: "r".into(),
            episode_id: "e".into(),
            action_id: "a".into(),
            simulation_tick: 20,
            simulation_time_ns: now,
            observation: ObservationRef {
                observation_id: "o".into(),
                capture_time_ns: now - age_ms * NS_PER_MS,
            },
            simulated_stop: false,
            proposed_action: ProposedAction::Pickup {
                target: "cube".into(),
                segment: "approach".into(),
            },
        }
    }

    #[test]
    fn stop_precedes_invalid_timestamp() {
        let mut p = proposal_at_age(0);
        p.simulated_stop = true;
        p.observation.capture_time_ns = p.simulation_time_ns + 1; // future
        assert_eq!(
            decide(&p, &Policy::default()),
            Decision::Reject {
                reason: RejectionReason::EmergencyStop
            }
        );
    }

    #[test]
    fn future_timestamp_rejected() {
        let mut p = proposal_at_age(0);
        p.observation.capture_time_ns = p.simulation_time_ns + NS_PER_MS;
        assert_eq!(
            decide(&p, &Policy::default()),
            Decision::Reject {
                reason: RejectionReason::InvalidTimestamp
            }
        );
    }
}
