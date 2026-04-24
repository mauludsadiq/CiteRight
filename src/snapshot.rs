use anyhow::Result;
use crate::models::LookupRecord;
use crate::hash::sha256_json;
use crate::courtlistener::{CourtListenerClient, CitationLookup};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Live CourtListener bridge (optional runtime mode)
/// Converts live API results → deterministic snapshot format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub snapshot_id: String,
    pub query: String,
    pub records: Vec<LookupRecord>,
    pub source: String,
    pub deterministic_basis: String,
}

pub fn fetch_live_snapshot(
    query: &str,
    client: &CourtListenerClient,
) -> Result<Snapshot> {
    let raw = client.lookup_text(query)?; // live API call

    let records = normalize_lookup_records(&raw);

    let mut snapshot = Snapshot {
        snapshot_id: "pending".to_string(),
        query: query.to_string(),
        records,
        source: "courtlistener:live".to_string(),
        deterministic_basis: "snapshot_v0".to_string(),
    };

    snapshot.snapshot_id = sha256_json(&snapshot)?;
    Ok(snapshot)
}

/// Normalize live API response into canonical LookupRecord format
fn normalize_lookup_records(raw: &[crate::models::LookupRecord]) -> Vec<LookupRecord> {
    raw.iter()
        .map(|r| LookupRecord {
                clusters: r.clusters.clone(),
                source: "live_normalized".to_string(),
                raw_response_digest: r.raw_response_digest.clone(),
                api_status: r.api_status,
                error_message: r.error_message.clone(),
                http_status: r.http_status,
                ..r.clone()
            })
        .collect()
}

/// Persist snapshot so it becomes replayable like fixtures
pub(crate) fn persist_snapshot(out_dir: &Path, snapshot: &Snapshot) -> Result<()> {
    std::fs::create_dir_all(out_dir)?;

    let path = out_dir.join(format!("snapshot_{}.json", snapshot.snapshot_id));
    let bytes = serde_json::to_vec_pretty(snapshot)?;

    std::fs::write(path, bytes)?;
    Ok(())
}
