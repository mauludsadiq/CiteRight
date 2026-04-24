use crate::models::LookupRecord;
use crate::hash::sha256_json;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseArtifact {
    pub artifact_id: String,
    pub canonical_id: String,
    pub case_name: String,
    pub citations: Vec<String>,
    pub date_filed: Option<String>,
    pub absolute_url: Option<String>,
    pub source: String,
    pub source_digest: String,
    pub verification_status: crate::models::VerificationStatus,
}

pub fn artifacts_from_lookup_results(results: &[LookupRecord]) -> Result<Vec<CaseArtifact>> {
    let mut out = Vec::new();

    for result in results {
        for cluster in &result.clusters {
            let (canonical_id, verification_status) = match cluster.id {
                Some(id) => (format!("cluster:{}", id), crate::models::VerificationStatus::Verified),
                None => (
                    format!("unverified:{}", result.raw_citation),
                    crate::models::VerificationStatus::Unverified { reason: crate::models::UnverifiedReason::NotFound },
                ),
            };

            let citations = cluster
                .citations
                .iter()
                .filter_map(|c| match (&c.volume, &c.reporter, &c.page) {
                    (Some(volume), Some(reporter), Some(page)) => Some(format!("{} {} {}", volume, reporter, page)),
                    _ => None,
                })
                .collect::<Vec<_>>();

            let mut artifact = CaseArtifact {
                artifact_id: "pending".to_string(),
                canonical_id,
                case_name: cluster.case_name.clone().unwrap_or_else(|| "UNKNOWN_CASE".to_string()),
                citations,
                date_filed: cluster.date_filed.clone(),
                absolute_url: cluster.absolute_url.clone(),
                source: result.source.clone(),
                source_digest: result.raw_response_digest.clone(),
                verification_status,
            };

            artifact.artifact_id = sha256_json(&artifact)?;
            out.push(artifact);
        }
    }

    Ok(dedup_artifacts(out))
}

fn dedup_artifacts(artifacts: Vec<CaseArtifact>) -> Vec<CaseArtifact> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();

    for artifact in artifacts {
        if seen.insert(artifact.canonical_id.clone()) {
            out.push(artifact);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_lookup_results_produce_no_artifacts() {
        let artifacts = artifacts_from_lookup_results(&[]).unwrap();
        assert!(artifacts.is_empty());
    }
    #[test]
    fn cluster_without_id_produces_unverified_artifact() {
        use crate::models::{CanonicalCluster, LookupRecord, VerificationStatus, UnverifiedReason};
        let record = LookupRecord {
            source: "test".to_string(),
            request_digest: "digest".to_string(),
            http_status: Some(200),
            api_status: Some(200),
            raw_citation: "Fake v. Case, 999 F.3d 1".to_string(),
            normalized_citations: vec![],
            clusters: vec![CanonicalCluster {
                id: None,
                case_name: Some("Fake v. Case".to_string()),
                absolute_url: None,
                date_filed: None,
                citations: vec![],
            }],
            error_message: "".to_string(),
            raw_response_digest: "digest".to_string(),
        };
        let artifacts = artifacts_from_lookup_results(&[record]).unwrap();
        assert_eq!(artifacts.len(), 1);
        assert!(matches!(
            artifacts[0].verification_status,
            VerificationStatus::Unverified { reason: UnverifiedReason::NotFound }
        ));
        assert!(artifacts[0].canonical_id.starts_with("unverified:"));
    }

    #[test]
    fn cluster_with_id_produces_verified_artifact() {
        use crate::models::{CanonicalCluster, LookupRecord, VerificationStatus};
        let record = LookupRecord {
            source: "test".to_string(),
            request_digest: "digest".to_string(),
            http_status: Some(200),
            api_status: Some(200),
            raw_citation: "Obergefell v. Hodges".to_string(),
            normalized_citations: vec![],
            clusters: vec![CanonicalCluster {
                id: Some(2812209),
                case_name: Some("Obergefell v. Hodges".to_string()),
                absolute_url: None,
                date_filed: None,
                citations: vec![],
            }],
            error_message: "".to_string(),
            raw_response_digest: "digest".to_string(),
        };
        let artifacts = artifacts_from_lookup_results(&[record]).unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].verification_status, VerificationStatus::Verified);
        assert_eq!(artifacts[0].canonical_id, "cluster:2812209");
    }

}
