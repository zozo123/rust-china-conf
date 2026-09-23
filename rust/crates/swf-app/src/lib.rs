//! swf-app: robot-demo session orchestration for the software factory demo.
//!
//! Owns: scenario identity, bridge subprocess lifecycle, the lock-step
//! decision protocol, decision logging, and session timeouts.

pub mod evidence;
pub mod protocol;
pub mod session;

pub use session::{Session, SessionConfig};
