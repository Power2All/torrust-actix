//! Serde serialization definition for AnnounceEvent.

use serde::{Deserialize, Serialize};
use crate::tracker::enums::announce_event::AnnounceEvent;

/// Serde remote definition for serializing [`AnnounceEvent`].
///
/// This enum is used with `#[serde(with = "AnnounceEventDef")]` to enable
/// JSON serialization of announce events. It maps the internal numeric
/// representation to human-readable string names.
///
/// # Usage
///
/// ```rust,ignore
/// use serde::Serialize;
/// use crate::tracker::enums::announce_event::AnnounceEvent;
/// use crate::tracker::enums::announce_event_def::AnnounceEventDef;
///
/// #[derive(Serialize)]
/// struct MyStruct {
///     #[serde(with = "AnnounceEventDef")]
///     event: AnnounceEvent,
/// }
/// ```
///
/// # Serialization Format
///
/// Events are serialized as their variant names: `"Started"`, `"Stopped"`,
/// `"Completed"`, or `"None"`.
#[derive(Serialize, Deserialize)]
#[serde(remote = "AnnounceEvent")]
pub enum AnnounceEventDef {
    /// Serializes as `"Started"`.
    Started,

    /// Serializes as `"Stopped"`.
    Stopped,

    /// Serializes as `"Completed"`.
    Completed,

    /// Serializes as `"None"`.
    None,
}