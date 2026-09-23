//! Wire types for the line-delimited JSON protocol between the Rust session
//! (parent) and the Python simulator bridge (child).
//!
//! Protocol messages travel alone on the child's stdout; all simulator
//! diagnostics go to stderr. One JSON object per line, both directions.

use robot_safety_gate::{Decision, Proposal};
use serde::{Deserialize, Serialize};

/// Child -> parent messages. The `type` tag is read first, then the line is
/// deserialized into the concrete payload. Unknown fields are ignored so the
/// bridge can attach presentation metadata without breaking the session.
#[derive(Debug, Deserialize)]
pub struct TypeTag {
    #[serde(rename = "type")]
    pub msg_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Hello {
    pub backend: String,
    pub version: u32,
    pub simulation_tick_ns: u64,
    /// Free-form backend label, e.g. "mock (NOT robosuite SIL)".
    #[serde(default)]
    pub backend_label: String,
}

/// A task-action proposal is exactly the gate's `Proposal` on the wire
/// (plus the `type` tag, which serde ignores during deserialization).
pub type ProposalMsg = Proposal;

#[derive(Debug, Clone, Deserialize)]
pub struct Outcome {
    pub action_id: String,
    pub simulation_tick: u64,
    pub dispatched: bool,
    pub action_kind: String,
    #[serde(default)]
    pub note: String,
}

/// Explicit hold step: physics may advance, but no task action is
/// dispatched. Holds are labeled so assertions can distinguish them from
/// pickup dispatches.
#[derive(Debug, Clone, Deserialize)]
pub struct HoldNotice {
    pub simulation_tick: u64,
    pub simulation_time_ns: u64,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EpisodeEnd {
    pub success: bool,
    pub reason: String,
    #[serde(default)]
    pub cube_height_m: f64,
    #[serde(default)]
    pub ticks: u64,
}

#[derive(Debug)]
pub enum BridgeMsg {
    Hello(Hello),
    Proposal(ProposalMsg),
    Outcome(Outcome),
    Hold(HoldNotice),
    EpisodeEnd(EpisodeEnd),
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("malformed JSON line: {0}")]
    MalformedJson(#[from] serde_json::Error),
    #[error("unknown message type: {0:?}")]
    UnknownType(String),
}

pub fn parse_bridge_line(line: &str) -> Result<BridgeMsg, ProtocolError> {
    let value: serde_json::Value = serde_json::from_str(line)?;
    let tag: TypeTag = serde_json::from_value(value.clone())?;
    match tag.msg_type.as_str() {
        "hello" => Ok(BridgeMsg::Hello(serde_json::from_value(value)?)),
        "proposal" => Ok(BridgeMsg::Proposal(serde_json::from_value(value)?)),
        "outcome" => Ok(BridgeMsg::Outcome(serde_json::from_value(value)?)),
        "hold" => Ok(BridgeMsg::Hold(serde_json::from_value(value)?)),
        "episode_end" => Ok(BridgeMsg::EpisodeEnd(serde_json::from_value(value)?)),
        other => Err(ProtocolError::UnknownType(other.to_string())),
    }
}

/// Parent -> child decision. Echoes run/episode/action/tick identifiers so an
/// approval is valid only for the exact proposed action at the exact tick.
#[derive(Debug, Serialize)]
pub struct DecisionMsg {
    #[serde(rename = "type")]
    pub msg_type: &'static str,
    pub run_id: String,
    pub episode_id: String,
    pub action_id: String,
    pub simulation_tick: u64,
    #[serde(flatten)]
    pub decision: Decision,
}

impl DecisionMsg {
    pub fn for_proposal(proposal: &Proposal, decision: Decision) -> Self {
        Self {
            msg_type: "decision",
            run_id: proposal.run_id.clone(),
            episode_id: proposal.episode_id.clone(),
            action_id: proposal.action_id.clone(),
            simulation_tick: proposal.simulation_tick,
            decision,
        }
    }
}
