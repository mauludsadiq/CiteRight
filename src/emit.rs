use crate::models::*;
use anyhow::Result;
use std::path::Path;

pub fn emit_verified_markdown(original: &str, records: &[VerificationRecord]) -> String {
    let mut out = String::new();
    let mut cursor = 0usize;
    let mut sorted = records.to_vec();
    sorted.sort_by_key(|r| r.claim.start_index);
    for r in sorted {
        if r.claim.start_index <= original.len() && cursor <= r.claim.start_index {
            out.push_str(&original[cursor..r.claim.start_index]);
        }
        match r.action {
            ClaimAction::Keep => out.push_str(&format!("{}<!-- CITE_RIGHT: VERIFIED {} -->", r.claim.raw, r.receipt_digest)),
            ClaimAction::ReplaceWithNormalized => {
                let replacement = r.lookup.normalized_citations.first().cloned().unwrap_or_else(|| r.claim.raw.clone());
                out.push_str(&format!("{}<!-- CITE_RIGHT: HEALED from '{}' {} -->", replacement, r.claim.raw, r.receipt_digest));
            }
            ClaimAction::BlockOrRedact => out.push_str(&format!("[CITATION BLOCKED: {} | {}]", r.claim.raw, r.reason)),
        }
        cursor = r.claim.end_index;
    }
    out.push_str(&original[cursor..]);
    out
}

pub fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    std::fs::write(path, bytes)?;
    Ok(())
}
