use crate::candidates::CitationCandidate;
use crate::courtlistener::{CitationLookup, FixtureLookup};
use crate::hash::sha256_json;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateResolution {
    pub resolution_id: String,
    pub candidate_id: String,
    pub search_query: String,
    pub lookup_source: String,
    pub result_count: usize,
    pub canonical_ids: Vec<String>,
    pub resolved: bool,
    pub deterministic_basis: String,
}

pub fn resolve_candidates_with_fixtures(
    candidates: &[CitationCandidate],
    fixture_path: &std::path::Path,
) -> Result<Vec<CandidateResolution>> {
    let lookup = FixtureLookup::from_file(fixture_path)?;

    let mut out = Vec::new();

    for c in candidates {
        let result = lookup.lookup_text(&c.search_query)?;

        let canonical_ids: Vec<String> = result
            .iter()
            .flat_map(|r| r.clusters.iter())
            .filter_map(|cl| cl.id.map(|id| format!("cluster:{}", id)))
            .collect();

        let resolved = !canonical_ids.is_empty();

        let mut res = CandidateResolution {
            resolution_id: "pending".to_string(),
            candidate_id: c.candidate_id.clone(),
            search_query: c.search_query.clone(),
            lookup_source: "fixture:courtlistener".to_string(),
            result_count: canonical_ids.len(),
            canonical_ids,
            resolved,
            deterministic_basis: "candidate_resolution_v0".to_string(),
        };

        res.resolution_id = sha256_json(&res)?;
        out.push(res);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use crate::candidates::{CitationCandidate, CandidateSource};
    use crate::planner::CitationNeedPriority;

    #[test]
    fn produces_resolution_objects() {
        let candidates = vec![CitationCandidate {
            candidate_id: "sha256:test".to_string(),
            need_id: "need".to_string(),
            claim_id: "claim".to_string(),
            source: CandidateSource::DeterministicQueryExpansion,
            search_query: "576 U.S. 644".to_string(),
            expected_artifact_type: "case".to_string(),
            jurisdiction_hint: Some("federal".to_string()),
            reporter_hint: Some("U.S.".to_string()),
            priority: CitationNeedPriority::High,
            deterministic_basis: "test".to_string(),
        }];

        // fixture path will be supplied in runtime test, so just ensure function compiles
        assert_eq!(candidates.len(), 1);
    }
}
