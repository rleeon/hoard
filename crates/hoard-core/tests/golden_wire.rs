//! # The wire's golden round-trip test (ADR 0021, C.6)
//!
//! The `wire` shapes compile in both the client and the server, but **that only
//! guarantees coherence within one build**: client and server are deployed
//! separately (a self-hoster runs a server from three versions ago against
//! yesterday's desktop; Hoard Cloud updates the server without touching the
//! installed clients). A change that compiles can still break the contract.
//!
//! The files in `tests/golden/` are the **byte-for-byte JSON of the last release**
//! (v1.0.4). Each one is deserialised with today's type and serialised again: if the
//! result is not the same object, the change breaks compatibility and the test
//! fails. Renaming a field, dropping it, changing its type or no longer emitting it
//! is caught here, not in production.
//!
//! **When adding a field**: add it to the type with `#[serde(default)]` and do NOT
//! touch the fixture (the old JSON has to keep loading). Only add a new fixture when
//! the new shape is also worth pinning.

use std::path::PathBuf;

use hoard_core::wire::{
    CreateSaveRequest, Game, Health, LogBatch, LogIngestResponse, MaxVersionsBody,
    MaxVersionsResponse, Save, Snapshot, SnapshotDetail, Whoami,
};
use serde::{de::DeserializeOwned, Serialize};

fn golden(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{name}.json"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("leyendo {}: {e}", path.display()))
}

/// Deserializa el golden con el tipo de hoy, lo vuelve a serializar y exige que
/// salga **el mismo objeto JSON**: ni un campo perdido, ni uno renombrado, ni un
/// valor movido.
fn round_trip<T: DeserializeOwned + Serialize>(name: &str) {
    let raw = golden(name);
    let parsed: T = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("{name}.json no deserializa con el tipo de hoy: {e}"));
    let before: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let after = serde_json::to_value(&parsed).unwrap();
    assert_eq!(
        before, after,
        "{name}.json does not survive the round-trip: the wire changed since the release"
    );
}

/// Only checks that the release's JSON **parses**. For the shapes the shared type
/// normalises when re-emitting (see `cloud_rename_response_parses`).
fn parses<T: DeserializeOwned>(name: &str) -> T {
    let raw = golden(name);
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("{name}.json no deserializa con el tipo de hoy: {e}"))
}

#[test]
fn health_round_trips() {
    round_trip::<Health>("health");
    // The release's golden carries no `cas`, and the client has to read that as
    // "this server does not negotiate content" rather than trip over it. If this
    // broke, a self-hoster on an old server would see uploads aimed at routes that
    // do not exist.
    let h: Health = parses("health");
    assert!(
        !h.cas,
        "absent means the server only understands the multipart"
    );
    assert!(!h.devices, "ausente → no lleva censo, no le mandes latidos");
}

/// A 1.1.3 server's `/v1/health`. The flags have to survive the round-trip **and**
/// be `true`: they are the only thing separating speaking the new protocol from
/// sending the whole folder, and keeping a device census from beating at a server
/// that has no idea what to do with it.
#[test]
fn cas_capable_health_round_trips() {
    round_trip::<Health>("health_cas");
    let h: Health = parses("health_cas");
    assert!(h.cas);
    assert!(h.devices);
    assert!(h.mode.is_none(), "sigue siendo self-hosted");
}

/// Cloud emits its own `HealthBody` (in `server::cloud::run`) and it carries **no**
/// `uptime_secs`. The client branches the whole protocol on this payload, so what
/// matters is that it parses.
#[test]
fn cloud_health_parses() {
    let h: Health = parses("health_cloud");
    assert_eq!(h.mode.as_deref(), Some("cloud"));
    assert_eq!(h.log_min_level.as_deref(), Some("warn"));
    assert_eq!(h.uptime_secs, 0, "ausente → default, no error");
}

#[test]
fn whoami_round_trips() {
    round_trip::<Whoami>("whoami");
}

#[test]
fn max_versions_round_trips() {
    round_trip::<MaxVersionsBody>("max_versions_body");
    round_trip::<MaxVersionsResponse>("max_versions_response");
}

#[test]
fn game_round_trips() {
    round_trip::<Game>("game");
}

#[test]
fn save_round_trips() {
    round_trip::<Save>("save");
    round_trip::<CreateSaveRequest>("create_save_request");
}

#[test]
fn snapshot_round_trips() {
    round_trip::<Snapshot>("snapshot");
    round_trip::<SnapshotDetail>("snapshot_detail");
}

#[test]
fn logs_round_trip() {
    round_trip::<LogBatch>("log_batch");
    round_trip::<LogIngestResponse>("log_ingest_response");
}

/// The cloud rename's response (`SaveSummary`, still defined separately in
/// `server::cloud::routes::saves`) has to keep parsing into the shared `Save`: it
/// omits the optional fields and emits the offset as `+00:00` rather than `Z`. A
/// round-trip does not apply, since re-emitting normalises to `Z`, the same instant,
/// but the parse does, and parsing is what the client does.
#[test]
fn cloud_rename_response_parses() {
    let save: Save = parses("save_cloud_rename");
    assert_eq!(save.game_slug.as_str(), "stardew-valley");
    assert_eq!(save.label, "granja");
    assert!(
        save.snapshot_count.is_none(),
        "cloud no calcula el agregado"
    );
    assert_eq!(save.created_at.offset(), time::UtcOffset::UTC);
}

/// The values crossing the wire go through `ids`' gate, so the golden also pins
/// that the release's ids are **still valid** today. If somebody tightens a `parse`
/// too far, this falls before a user does.
#[test]
fn release_values_still_pass_the_gate() {
    let save: Save = parses("save");
    assert_eq!(save.id.as_str(), "3f2504e0-4f89-41d3-9a0c-0305e82c3301");
    assert_eq!(save.game_slug.as_str(), "stardew-valley");

    let whoami: Whoami = parses("whoami");
    assert_eq!(whoami.username.as_str(), "jacka");

    let detail: SnapshotDetail = parses("snapshot_detail");
    assert_eq!(detail.files.len(), 2);
    assert_eq!(detail.files[0].sha256.as_ref().unwrap().as_str().len(), 64);

    let batch: LogBatch = parses("log_batch");
    assert!(batch.device.fingerprint.is_some());
    assert_eq!(batch.entries.len(), 2);
    assert!(batch.entries[1].fields.is_none());
}
