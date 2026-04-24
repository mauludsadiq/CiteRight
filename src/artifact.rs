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
}

pub fn artifacts_from_lookup_results(results: &[LookupRecord]) -> Result<Vec<CaseArtifact>> {
    let mut out = Vec::new();

    for result in results {
        for cluster in &result.clusters {
            let canonical_id = match cluster.id {
                Some(id) => format!("cluster:{}", id),
                None => continue,
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
}
