use crate::reasoning::applicability::{ApplicabilityScore, ApplicabilityVerdict};
use crate::reasoning::argument_graph::ArgumentGraph;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub overall: OverallValidity,
    pub claim_validations: Vec<ClaimValidation>,
    pub unsupported_claims: Vec<String>,
    pub weakly_supported_claims: Vec<String>,
    pub conflicting_authority: Vec<ConflictingAuthority>,
    pub summary: String,
    pub validation_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OverallValidity {
    Valid,
    PartiallyValid,
    Invalid,
    Indeterminate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimValidation {
    pub claim_id: String,
    pub claim_text: String,
    pub validity: ClaimValidity,
    pub supporting_cases: Vec<String>,
    pub applicability_score: f32,
    pub llm_assessment: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClaimValidity {
    Supported,
    PartiallySupported,
    Unsupported,
    Unverifiable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictingAuthority {
    pub claim_id: String,
    pub supporting_case: String,
    pub conflicting_case: String,
    pub explanation: String,
}

pub fn validate_argument(
    graph: &ArgumentGraph,
    applicability_scores: &[ApplicabilityScore],
) -> ValidationReport {
    let mut claim_validations = Vec::new();
    let mut unsupported_claims = Vec::new();
    let mut weakly_supported_claims = Vec::new();
    let mut conflicting_authority = Vec::new();

    for claim in &graph.claims {
        let supporting_cases = graph.cases_supporting_claim(&claim.claim_id);

        // Find best applicability score for this claim's cited cases
        let best_applicability = claim.cited_case_ids.iter()
            .filter_map(|id| applicability_scores.iter().find(|s| &s.canonical_id == id))
            .map(|s| s.score)
            .fold(0.0_f32, f32::max);

        // Determine validity from LLM assessment + applicability
        let (validity, reason) = determine_claim_validity(
            &claim.assessment,
            best_applicability,
            supporting_cases.len(),
        );

        // Check for conflicting authority
        for case_id in &claim.cited_case_ids {
            let conflicts = graph.conflicting_cases(case_id);
            for conflict in conflicts {
                conflicting_authority.push(ConflictingAuthority {
                    claim_id: claim.claim_id.clone(),
                    supporting_case: case_id.clone(),
                    conflicting_case: conflict.canonical_id.clone(),
                    explanation: format!(
                        "{} may conflict with {} on this point",
                        case_id, conflict.canonical_id
                    ),
                });
            }
        }

        match &validity {
            ClaimValidity::Unsupported => unsupported_claims.push(claim.claim_id.clone()),
            ClaimValidity::PartiallySupported => weakly_supported_claims.push(claim.claim_id.clone()),
            _ => {}
        }

        claim_validations.push(ClaimValidation {
            claim_id: claim.claim_id.clone(),
            claim_text: claim.text.clone(),
            validity,
            supporting_cases: supporting_cases.iter().map(|c| c.case_name.clone()).collect(),
            applicability_score: best_applicability,
            llm_assessment: claim.assessment.clone(),
            reason,
        });
    }

    let overall = determine_overall_validity(&claim_validations, &conflicting_authority);
    let summary = build_summary(&overall, &claim_validations, &unsupported_claims, &conflicting_authority);

    let mut report = ValidationReport {
        overall,
        claim_validations,
        unsupported_claims,
        weakly_supported_claims,
        conflicting_authority,
        summary,
        validation_digest: String::new(),
    };

    report.validation_digest = compute_digest(&report);
    report
}

fn determine_claim_validity(
    assessment: &Option<String>,
    applicability: f32,
    supporting_count: usize,
) -> (ClaimValidity, String) {
    if supporting_count == 0 {
        return (ClaimValidity::Unsupported, "No verified supporting cases found".to_string());
    }

    // Parse LLM assessment if present
    let llm_supported = assessment.as_ref().map(|a| {
        let a_lower = a.to_lowercase();
        if a_lower.contains("unsupported") {
            0
        } else if a_lower.contains("partially") {
            1
        } else if a_lower.contains("supported") {
            2
        } else {
            -1
        }
    });

    match (llm_supported, applicability) {
        (Some(2), a) if a >= 0.3 => (
            ClaimValidity::Supported,
            format!("LLM confirms claim is supported; applicability score {:.2}", a),
        ),
        (Some(2), a) => (
            ClaimValidity::PartiallySupported,
            format!("LLM confirms support but applicability is low ({:.2})", a),
        ),
        (Some(1), _) => (
            ClaimValidity::PartiallySupported,
            "LLM assessment indicates partial support".to_string(),
        ),
        (Some(0), _) => (
            ClaimValidity::Unsupported,
            "LLM assessment indicates claim is not supported by cited authority".to_string(),
        ),
        (None, a) if a >= 0.5 => (
            ClaimValidity::PartiallySupported,
            format!("No LLM assessment; applicability score suggests relevance ({:.2})", a),
        ),
        (None, a) if a >= 0.2 => (
            ClaimValidity::PartiallySupported,
            format!("No LLM assessment; weak applicability ({:.2})", a),
        ),
        _ => (
            ClaimValidity::Unverifiable,
            "Insufficient signal to validate claim".to_string(),
        ),
    }
}

fn determine_overall_validity(
    validations: &[ClaimValidation],
    conflicts: &[ConflictingAuthority],
) -> OverallValidity {
    if validations.is_empty() {
        return OverallValidity::Indeterminate;
    }

    let total = validations.len();
    let supported = validations.iter()
        .filter(|v| v.validity == ClaimValidity::Supported)
        .count();
    let unsupported = validations.iter()
        .filter(|v| v.validity == ClaimValidity::Unsupported)
        .count();

    if !conflicts.is_empty() || unsupported > 0 {
        if unsupported == total {
            OverallValidity::Invalid
        } else {
            OverallValidity::PartiallyValid
        }
    } else if supported == total {
        OverallValidity::Valid
    } else {
        OverallValidity::PartiallyValid
    }
}

fn build_summary(
    overall: &OverallValidity,
    validations: &[ClaimValidation],
    unsupported: &[String],
    conflicts: &[ConflictingAuthority],
) -> String {
    let total = validations.len();
    let supported_count = validations.iter()
        .filter(|v| v.validity == ClaimValidity::Supported)
        .count();

    let mut parts = vec![
        format!("Overall validity: {:?}", overall),
        format!("{}/{} claims supported by verified authority", supported_count, total),
    ];

    if !unsupported.is_empty() {
        parts.push(format!("{} claim(s) unsupported: {}", unsupported.len(), unsupported.join(", ")));
    }

    if !conflicts.is_empty() {
        parts.push(format!("{} conflicting authority relationship(s) detected", conflicts.len()));
    }

    parts.join(". ")
}

fn compute_digest(report: &ValidationReport) -> String {
    use sha2::{Sha256, Digest};
    let json = serde_json::to_string(&(&report.overall, &report.claim_validations, &report.unsupported_claims))
        .unwrap_or_default();
    let hash = Sha256::digest(json.as_bytes());
    format!("sha256:{}", hex::encode(hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reasoning::applicability::{ApplicabilityScore, ApplicabilitySignal, ApplicabilityVerdict};
    use crate::reasoning::argument_graph::{ArgumentGraph, CaseNode};
    use crate::reasoning::holdings_extractor::{HoldingNode, HoldingConfidence, ExtractedHolding};

    fn mock_holding(id: &str, name: &str, holding: &str) -> HoldingNode {
        HoldingNode {
            canonical_id: id.to_string(),
            case_name: name.to_string(),
            holdings: vec![ExtractedHolding {
                text: holding.to_string(),
                trigger: "We hold".to_string(),
                confidence: HoldingConfidence::High,
            }],
            opinion_url: None,
            extraction_method: "pattern_v1".to_string(),
        }
    }

    fn mock_score(id: &str, score: f32) -> ApplicabilityScore {
        ApplicabilityScore {
            canonical_id: id.to_string(),
            case_name: "Test Case".to_string(),
            score,
            signals: vec![],
            verdict: if score >= 0.65 { ApplicabilityVerdict::HighlyApplicable }
                     else if score >= 0.40 { ApplicabilityVerdict::ModeratelyApplicable }
                     else { ApplicabilityVerdict::WeaklyApplicable },
        }
    }

    #[test]
    fn validates_supported_argument() {
        let mut graph = ArgumentGraph::new();
        let h = mock_holding("cluster:1", "Obergefell v. Hodges", "We hold that same-sex couples may marry.");
        graph.add_node_from_holding(&h);
        graph.add_claim("cluster:1", "Constitutional rights apply equally.", vec!["cluster:1".to_string()]);
        graph.set_claim_assessment("cluster:1", "Supported: claim accurately reflects holding");
        graph.finalize();

        let scores = vec![mock_score("cluster:1", 0.75)];
        let report = validate_argument(&graph, &scores);
        assert_eq!(report.overall, OverallValidity::Valid);
        assert!(report.validation_digest.starts_with("sha256:"));
    }

    #[test]
    fn flags_unsupported_claims() {
        let mut graph = ArgumentGraph::new();
        let h = mock_holding("cluster:2", "Conti v. Perdue", "Commodity forward agreements need not trade on exchange.");
        graph.add_node_from_holding(&h);
        graph.add_claim("cluster:2", "Constitutional due process applies.", vec!["cluster:2".to_string()]);
        graph.set_claim_assessment("cluster:2", "Unsupported: claim does not match holding");
        graph.finalize();

        let scores = vec![mock_score("cluster:2", 0.1)];
        let report = validate_argument(&graph, &scores);
        assert!(!report.unsupported_claims.is_empty());
    }

    #[test]
    fn detects_conflicting_authority() {
        use crate::reasoning::argument_graph::EdgeType;
        let mut graph = ArgumentGraph::new();
        let h1 = mock_holding("cluster:1", "Case A", "We hold X applies.");
        let h2 = mock_holding("cluster:2", "Case B", "We hold X does not apply.");
        graph.add_node_from_holding(&h1);
        graph.add_node_from_holding(&h2);
        graph.add_claim("cluster:1", "X applies here.", vec!["cluster:1".to_string()]);
        graph.add_edge("cluster:1", "cluster:2", EdgeType::Contradicts, 0.8, None);
        graph.finalize();

        let scores = vec![mock_score("cluster:1", 0.7)];
        let report = validate_argument(&graph, &scores);
        assert!(!report.conflicting_authority.is_empty());
    }

    #[test]
    fn produces_digest_on_empty_graph() {
        let graph = ArgumentGraph::new();
        let report = validate_argument(&graph, &[]);
        assert_eq!(report.overall, OverallValidity::Indeterminate);
        assert!(report.validation_digest.starts_with("sha256:"));
    }
}
