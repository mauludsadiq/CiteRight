use crate::candidates::CitationCandidate;
use crate::hash::sha256_json;
use crate::planner::CitationNeed;
use crate::resolver::CandidateResolution;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedCitation {
    pub selection_id: String,
    pub need_id: String,
    pub candidate_id: String,
    pub resolution_id: String,
    pub canonical_ids: Vec<String>,
    pub selected: bool,
    pub reason: String,
    pub deterministic_basis: String,
}

pub fn select_best_candidates(
    needs: &[CitationNeed],
    candidates: &[CitationCandidate],
    resolutions: &[CandidateResolution],
) -> Result<Vec<SelectedCitation>> {
    let mut out = Vec::new();

    let res_map: HashMap<_, _> = resolutions
        .iter()
        .map(|r| (r.candidate_id.clone(), r))
        .collect();

    for need in needs {
        let mut best: Option<(&CitationCandidate, &CandidateResolution)> = None;

        for c in candidates.iter().filter(|c| c.need_id == need.need_id) {
            if let Some(r) = res_map.get(&c.candidate_id) {
                if r.resolved {
                    match best {
                        None => best = Some((c, *r)),
                        Some((_, best_r)) => {
                            if r.result_count < best_r.result_count {
                                best = Some((c, *r));
                            }
                        }
                    }
                }
            }
        }

        let (candidate_id, resolution_id, canonical_ids, selected, reason) = match best {
            Some((c, r)) => (
                c.candidate_id.clone(),
                r.resolution_id.clone(),
                r.canonical_ids.clone(),
                true,
                "unique or minimal match".to_string(),
            ),
            None => (
                "none".to_string(),
                "none".to_string(),
                vec![],
                false,
                "no resolved candidate".to_string(),
            ),
        };

        let mut sel = SelectedCitation {
            selection_id: "pending".to_string(),
            need_id: need.need_id.clone(),
            candidate_id,
            resolution_id,
            canonical_ids,
            selected,
            reason,
            deterministic_basis: "selection_v0_min_result_count".to_string(),
        };

        sel.selection_id = sha256_json(&sel)?;
        out.push(sel);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner::{CitationNeed, CitationNeedKind, CitationNeedPriority};

    #[test]
    fn selects_resolved_candidate() {
        let needs = vec![CitationNeed {
            need_id: "need1".to_string(),
            claim_id: "c".to_string(),
            kind: CitationNeedKind::SupportHolding,
            priority: CitationNeedPriority::High,
            query_text: "test".to_string(),
            required_artifact: "case".to_string(),
            deterministic_basis: "t".to_string(),
        }];

        let candidates = vec![CitationCandidate {
            candidate_id: "cand1".to_string(),
            need_id: "need1".to_string(),
            claim_id: "c".to_string(),
            source: crate::candidates::CandidateSource::DeterministicQueryExpansion,
            search_query: "test".to_string(),
            expected_artifact_type: "case".to_string(),
            jurisdiction_hint: None,
            reporter_hint: None,
            priority: CitationNeedPriority::High,
            deterministic_basis: "t".to_string(),
        }];

        let resolutions = vec![CandidateResolution {
            resolution_id: "res1".to_string(),
            candidate_id: "cand1".to_string(),
            search_query: "test".to_string(),
            lookup_source: "fixture".to_string(),
            result_count: 1,
            canonical_ids: vec!["cluster:1".to_string()],
            resolved: true,
            deterministic_basis: "t".to_string(),
        }];

        let sel = select_best_candidates(&needs, &candidates, &resolutions).unwrap();
        assert!(sel[0].selected);
    }
}
