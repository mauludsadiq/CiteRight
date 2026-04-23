use crate::claims::{ClaimKind, LegalClaim};
use crate::hash::sha256_json;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CitationNeedKind {
    VerifyNamedAuthority,
    SupportHolding,
    SupportRule,
    SupportFactualAssertion,
    ReviewOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CitationNeedPriority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationNeed {
    pub need_id: String,
    pub claim_id: String,
    pub kind: CitationNeedKind,
    pub priority: CitationNeedPriority,
    pub query_text: String,
    pub required_artifact: String,
    pub deterministic_basis: String,
}

pub fn plan_citation_needs(claims: &[LegalClaim]) -> Result<Vec<CitationNeed>> {
    let mut needs = Vec::new();

    for claim in claims {
        let (kind, priority, required_artifact) = match claim.kind {
            ClaimKind::NamedAuthority => (
                CitationNeedKind::VerifyNamedAuthority,
                CitationNeedPriority::Critical,
                "canonical case artifact",
            ),
            ClaimKind::HoldingClaim => (
                CitationNeedKind::SupportHolding,
                CitationNeedPriority::High,
                "case holding or quoted proposition",
            ),
            ClaimKind::RuleStatement => (
                CitationNeedKind::SupportRule,
                CitationNeedPriority::High,
                "binding or persuasive authority",
            ),
            ClaimKind::FactualAssertion => (
                CitationNeedKind::SupportFactualAssertion,
                CitationNeedPriority::Medium,
                "record cite, exhibit, statute, regulation, or public source",
            ),
            ClaimKind::NeedsAuthority => (
                CitationNeedKind::ReviewOnly,
                CitationNeedPriority::Low,
                "human review or optional authority",
            ),
        };

        let mut need = CitationNeed {
            need_id: "pending".to_string(),
            claim_id: claim.claim_id.clone(),
            kind,
            priority,
            query_text: claim.text.clone(),
            required_artifact: required_artifact.to_string(),
            deterministic_basis: "claim_kind_to_citation_need_v0".to_string(),
        };

        need.need_id = sha256_json(&need)?;
        needs.push(need);
    }

    Ok(needs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claims::{ClaimKind, LegalClaim};

    #[test]
    fn named_authority_becomes_critical_verification_need() {
        let claims = vec![LegalClaim {
            claim_id: "sha256:test".to_string(),
            kind: ClaimKind::NamedAuthority,
            text: "In re Example Corp., 1 B.R. 1".to_string(),
            start_index: 0,
            end_index: 30,
            confidence_basis: "test".to_string(),
        }];

        let needs = plan_citation_needs(&claims).unwrap();
        assert_eq!(needs.len(), 1);
        assert_eq!(needs[0].kind, CitationNeedKind::VerifyNamedAuthority);
        assert_eq!(needs[0].priority, CitationNeedPriority::Critical);
    }
}
