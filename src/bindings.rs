use crate::artifact::CaseArtifact;
use crate::hash::sha256_json;
use crate::selector::SelectedCitation;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedArtifactBinding {
    pub binding_id: String,
    pub selection_id: String,
    pub need_id: String,
    pub canonical_id: String,
    pub artifact_id: String,
    pub case_name: String,
    pub citations: Vec<String>,
    pub date_filed: Option<String>,
    pub absolute_url: Option<String>,
    pub bound: bool,
    pub verification_status: crate::models::VerificationStatus,
    pub reason: String,
    pub deterministic_basis: String,
}

pub fn bind_selections_to_artifacts(
    selections: &[SelectedCitation],
    artifacts: &[CaseArtifact],
) -> Result<Vec<SelectedArtifactBinding>> {
    let artifact_map: HashMap<_, _> = artifacts
        .iter()
        .map(|a| (a.canonical_id.clone(), a))
        .collect();

    let mut out = Vec::new();

    for selection in selections {
        if !selection.selected {
            continue;
        }

        for canonical_id in &selection.canonical_ids {
            let mut binding = if let Some(artifact) = artifact_map.get(canonical_id) {
                SelectedArtifactBinding {
                    binding_id: "pending".to_string(),
                    selection_id: selection.selection_id.clone(),
                    need_id: selection.need_id.clone(),
                    canonical_id: canonical_id.clone(),
                    artifact_id: artifact.artifact_id.clone(),
                    case_name: artifact.case_name.clone(),
                    citations: artifact.citations.clone(),
                    date_filed: artifact.date_filed.clone(),
                    absolute_url: artifact.absolute_url.clone(),
                    bound: true,
                    verification_status: artifact.verification_status.clone(),
                    reason: "canonical artifact found".to_string(),
                    deterministic_basis: "selection_artifact_binding_v0".to_string(),
                }
            } else {
                SelectedArtifactBinding {
                    binding_id: "pending".to_string(),
                    selection_id: selection.selection_id.clone(),
                    need_id: selection.need_id.clone(),
                    canonical_id: canonical_id.clone(),
                    artifact_id: "none".to_string(),
                    case_name: "UNBOUND".to_string(),
                    citations: vec![],
                    date_filed: None,
                    absolute_url: None,
                    bound: false,
                    verification_status: crate::models::VerificationStatus::Unverified { reason: crate::models::UnverifiedReason::NotFound },
                    reason: "selected canonical id has no artifact".to_string(),
                    deterministic_basis: "selection_artifact_binding_v0".to_string(),
                }
            };

            binding.binding_id = sha256_json(&binding)?;
            out.push(binding);
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_inputs_produce_empty_bindings() {
        let bindings = bind_selections_to_artifacts(&[], &[]).unwrap();
        assert!(bindings.is_empty());
    }
}
