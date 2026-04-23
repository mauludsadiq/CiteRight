use crate::hash::sha256_json;
use crate::models::*;
use std::collections::HashMap;

pub fn verify_claims(claims: Vec<CitationClaim>, lookups: Vec<LookupRecord>, policy: &GatePolicy) -> anyhow::Result<Vec<VerificationRecord>> {
    let mut by_raw: HashMap<String, LookupRecord> = HashMap::new();
    for l in lookups { by_raw.insert(l.raw_citation.clone(), l); }
    let mut out = Vec::new();
    for claim in claims {
        let lookup = by_raw.remove(&claim.raw).unwrap_or_else(|| missing_lookup(&claim.raw));
        let (status, action, reason) = classify(&claim, &lookup, policy);
        let mut rec = VerificationRecord { claim, lookup, status, action, reason, receipt_digest: String::new() };
        rec.receipt_digest = sha256_json(&rec)?;
        out.push(rec);
    }
    Ok(out)
}

fn classify(claim: &CitationClaim, lookup: &LookupRecord, policy: &GatePolicy) -> (ClaimStatus, ClaimAction, String) {
    match lookup.api_status.unwrap_or(0) {
        200 => {
            if policy.require_single_cluster && lookup.clusters.len() != 1 {
                return (ClaimStatus::Ambiguous, ClaimAction::BlockOrRedact, format!("expected exactly one canonical cluster; got {}", lookup.clusters.len()));
            }
            if policy.heal_unique_normalized && lookup.normalized_citations.len() == 1 && lookup.normalized_citations[0] != claim.raw {
                return (ClaimStatus::Healed, ClaimAction::ReplaceWithNormalized, "unique normalized citation found".to_string());
            }
            (ClaimStatus::Verified, ClaimAction::Keep, "canonical citation found".to_string())
        }
        300 => (ClaimStatus::Ambiguous, ClaimAction::BlockOrRedact, "multiple canonical candidates".to_string()),
        400 => (ClaimStatus::Blocked, ClaimAction::BlockOrRedact, "invalid reporter or malformed citation".to_string()),
        404 => (ClaimStatus::Blocked, ClaimAction::BlockOrRedact, "no canonical source found".to_string()),
        429 => (ClaimStatus::Blocked, ClaimAction::BlockOrRedact, "lookup throttled or too many citations".to_string()),
        other => (ClaimStatus::Blocked, ClaimAction::BlockOrRedact, format!("unresolved lookup status {other}")),
    }
}

fn missing_lookup(raw: &str) -> LookupRecord {
    LookupRecord {
        source: "local:missing".to_string(),
        request_digest: "sha256:missing".to_string(),
        http_status: None,
        api_status: Some(404),
        raw_citation: raw.to_string(),
        normalized_citations: vec![raw.to_string()],
        clusters: vec![],
        error_message: "citation not returned by lookup source".to_string(),
        raw_response_digest: "sha256:missing".to_string(),
    }
}

pub fn counts(records: &[VerificationRecord]) -> RunCounts {
    RunCounts {
        extracted_citations: records.len(),
        verified: records.iter().filter(|r| r.status == ClaimStatus::Verified).count(),
        healed: records.iter().filter(|r| r.status == ClaimStatus::Healed).count(),
        ambiguous: records.iter().filter(|r| r.status == ClaimStatus::Ambiguous).count(),
        blocked: records.iter().filter(|r| r.status == ClaimStatus::Blocked).count(),
    }
}
