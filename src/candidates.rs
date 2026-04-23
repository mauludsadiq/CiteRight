use crate::hash::sha256_json;
use crate::planner::{CitationNeed, CitationNeedKind, CitationNeedPriority};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CandidateSource {
    DeterministicQueryExpansion,
    AiProposed,
    UserProvided,
    CanonicalLookup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationCandidate {
    pub candidate_id: String,
    pub need_id: String,
    pub claim_id: String,
    pub source: CandidateSource,
    pub search_query: String,
    pub expected_artifact_type: String,
    pub jurisdiction_hint: Option<String>,
    pub reporter_hint: Option<String>,
    pub priority: CitationNeedPriority,
    pub deterministic_basis: String,
}

pub fn generate_candidates(needs: &[CitationNeed]) -> Result<Vec<CitationCandidate>> {
    let mut candidates = Vec::new();

    for need in needs {
        let expected_artifact_type = match need.kind {
            CitationNeedKind::VerifyNamedAuthority => "case",
            CitationNeedKind::SupportHolding => "case_holding",
            CitationNeedKind::SupportRule => "case_or_statute",
            CitationNeedKind::SupportFactualAssertion => "record_or_public_source",
            CitationNeedKind::ReviewOnly => "review_note",
        };

        let mut queries = vec![normalize_query(&need.query_text)];

        if matches!(need.kind, CitationNeedKind::SupportHolding | CitationNeedKind::SupportRule) {
            queries.push(strip_citation_noise(&need.query_text));
        }

        for query in queries.into_iter().filter(|q| !q.trim().is_empty()) {
            let mut candidate = CitationCandidate {
                candidate_id: "pending".to_string(),
                need_id: need.need_id.clone(),
                claim_id: need.claim_id.clone(),
                source: CandidateSource::DeterministicQueryExpansion,
                search_query: query,
                expected_artifact_type: expected_artifact_type.to_string(),
                jurisdiction_hint: infer_jurisdiction_hint(&need.query_text),
                reporter_hint: infer_reporter_hint(&need.query_text),
                priority: need.priority.clone(),
                deterministic_basis: "citation_candidate_expansion_v0".to_string(),
            };

            candidate.candidate_id = sha256_json(&candidate)?;
            candidates.push(candidate);
        }
    }

    Ok(dedup_candidates(candidates))
}

fn normalize_query(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_citation_noise(text: &str) -> String {
    let mut out = Vec::new();

    for token in text.split_whitespace() {
        let t = token.trim_matches(|c: char| c == ',' || c == ';' || c == '(' || c == ')');

        let looks_numeric = t.chars().all(|c| c.is_ascii_digit());
        let looks_reporter = matches!(
            t,
            "U.S." | "US" | "S.Ct." | "L.Ed." | "F.2d" | "F.3d" | "F.Supp." | "B.R."
        );

        if !looks_numeric && !looks_reporter {
            out.push(t);
        }
    }

    out.join(" ")
}

fn infer_jurisdiction_hint(text: &str) -> Option<String> {
    let lower = text.to_lowercase();

    if lower.contains("u.s.") || lower.contains("supreme court") || lower.contains("constitutional") {
        Some("federal".to_string())
    } else if lower.contains("bankruptcy") || lower.contains("b.r.") {
        Some("federal_bankruptcy".to_string())
    } else {
        None
    }
}

fn infer_reporter_hint(text: &str) -> Option<String> {
    if text.contains("U.S.") || text.contains(" US ") {
        Some("U.S.".to_string())
    } else if text.contains("B.R.") {
        Some("B.R.".to_string())
    } else {
        None
    }
}

fn dedup_candidates(candidates: Vec<CitationCandidate>) -> Vec<CitationCandidate> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();

    for candidate in candidates {
        let key = format!(
            "{}|{}|{}",
            candidate.need_id, candidate.search_query, candidate.expected_artifact_type
        );

        if seen.insert(key) {
            out.push(candidate);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner::{CitationNeed, CitationNeedKind, CitationNeedPriority};

    #[test]
    fn generates_candidates_from_support_need() {
        let needs = vec![CitationNeed {
            need_id: "sha256:need".to_string(),
            claim_id: "sha256:claim".to_string(),
            kind: CitationNeedKind::SupportHolding,
            priority: CitationNeedPriority::High,
            query_text: "Obergefell v. Hodges, 576 U.S. 644, confirmed equal protection.".to_string(),
            required_artifact: "case holding or quoted proposition".to_string(),
            deterministic_basis: "test".to_string(),
        }];

        let candidates = generate_candidates(&needs).unwrap();
        assert!(!candidates.is_empty());
        assert_eq!(candidates[0].expected_artifact_type, "case_holding");
    }
}
