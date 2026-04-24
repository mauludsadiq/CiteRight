use crate::candidates::CitationCandidate;
use crate::claims::LegalClaim;
use crate::hash::sha256_json;
use crate::planner::CitationNeed;
use crate::resolver::CandidateResolution;
use crate::selector::SelectedCitation;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttorneyAttestation {
    pub attorney_name: Option<String>,
    pub bar_number: Option<String>,
    pub jurisdiction: Option<String>,
    pub attestation_date: DateTime<Utc>,
    pub attestation_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAuditReceipt {
    pub audit_id: String,
    pub tool: String,
    pub created_at: DateTime<Utc>,
    pub input_path: String,
    pub input_digest: String,
    pub claims_count: usize,
    pub needs_count: usize,
    pub candidates_count: usize,
    pub resolutions_count: usize,
    pub selections_count: usize,
    pub selected_count: usize,
    pub unresolved_count: usize,
    pub verified_count: usize,
    pub unverified_count: usize,
    pub claims_digest: String,
    pub needs_digest: String,
    pub candidates_digest: String,
    pub resolutions_digest: String,
    pub selections_digest: String,
    pub attestation: AttorneyAttestation,
    pub receipt_digest: String,
}

pub struct AttestationParams {
    pub attorney_name: Option<String>,
    pub bar_number: Option<String>,
    pub jurisdiction: Option<String>,
    pub verified_count: usize,
    pub unverified_count: usize,
}

#[allow(clippy::too_many_arguments)]
pub fn write_ai_audit(
    out: &Path,
    input_path: &Path,
    claims: &[LegalClaim],
    needs: &[CitationNeed],
    candidates: &[CitationCandidate],
    resolutions: &[CandidateResolution],
    selections: &[SelectedCitation],
    attestation_params: AttestationParams,
) -> Result<AiAuditReceipt> {
    std::fs::create_dir_all(out)?;

    write_pretty(out.join("claims.json").as_path(), claims)?;
    write_pretty(out.join("needs.json").as_path(), needs)?;
    write_pretty(out.join("candidates.json").as_path(), candidates)?;
    write_pretty(out.join("resolutions.json").as_path(), resolutions)?;
    write_pretty(out.join("selections.json").as_path(), selections)?;

    let selected_count = selections.iter().filter(|s| s.selected).count();
    let unresolved_count = selections.iter().filter(|s| !s.selected).count();

    let input_bytes = std::fs::read(input_path).unwrap_or_default();
    let input_digest = {
        use sha2::{Sha256, Digest};
        let hash = Sha256::digest(&input_bytes);
        format!("sha256:{}", hex::encode(hash))
    };

    let now = Utc::now();
    let attestation_text = format!(
        "I, {}, Bar No. {}, {}, hereby attest that the citations in document {} (sha256 digest: {}) were verified using CiteRight on {}. Verified: {}. Unverified: {}.",
        attestation_params.attorney_name.as_deref().unwrap_or("[ATTORNEY NAME]"),
        attestation_params.bar_number.as_deref().unwrap_or("[BAR NUMBER]"),
        attestation_params.jurisdiction.as_deref().unwrap_or("[JURISDICTION]"),
        input_path.display(),
        input_digest,
        now.format("%Y-%m-%d %H:%M:%S UTC"),
        attestation_params.verified_count,
        attestation_params.unverified_count,
    );

    let attestation = AttorneyAttestation {
        attorney_name: attestation_params.attorney_name,
        bar_number: attestation_params.bar_number,
        jurisdiction: attestation_params.jurisdiction,
        attestation_date: now,
        attestation_text,
    };

    let mut receipt = AiAuditReceipt {
        audit_id: "pending".to_string(),
        tool: "Cite Right AI Audit".to_string(),
        created_at: now,
        input_path: input_path.display().to_string(),
        input_digest,
        claims_count: claims.len(),
        needs_count: needs.len(),
        candidates_count: candidates.len(),
        resolutions_count: resolutions.len(),
        selections_count: selections.len(),
        selected_count,
        unresolved_count,
        verified_count: attestation_params.verified_count,
        unverified_count: attestation_params.unverified_count,
        claims_digest: sha256_json(&claims)?,
        needs_digest: sha256_json(&needs)?,
        candidates_digest: sha256_json(&candidates)?,
        resolutions_digest: sha256_json(&resolutions)?,
        selections_digest: sha256_json(&selections)?,
        attestation,
        receipt_digest: "pending".to_string(),
    };

    receipt.audit_id = sha256_json(&(
        &receipt.claims_digest,
        &receipt.needs_digest,
        &receipt.candidates_digest,
        &receipt.resolutions_digest,
        &receipt.selections_digest,
        &receipt.input_digest,
    ))?;

    receipt.receipt_digest = sha256_json(&receipt)?;
    write_pretty(out.join("ai_audit_receipt.json").as_path(), &receipt)?;

    Ok(receipt)
}

fn write_pretty<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    std::fs::write(path, bytes)?;
    Ok(())
}
