//! An alias for the shared supervisor; the implementation lives in
//! `hoard_agent::supervisor` (ADR 0021).
//!
//! It lives there because the daemon (`hoardd`) has to satisfy the same D.12 rule
//! ("if it outlives a request, it goes under `supervise`") and cannot use a module
//! private to the desktop. This alias stays so the path the ADR names,
//! `commands/supervisor.rs`, is still what anybody looking for where a desktop task
//! gets supervised will find.

pub use hoard_agent::supervisor::{supervise, Finished};
