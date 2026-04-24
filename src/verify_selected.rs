use crate::selector::SelectedCitation;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedClaim {
    pub claim_id: String,
    pub canonical_id: String,
    pub source: String,
}

pub fn selected_to_claims(selections: &[SelectedCitation]) -> Result<Vec<SelectedClaim>> {
    let mut out = Vec::new();

    for s in selections.iter().filter(|s| s.selected) {
        for cid in &s.canonical_ids {
            let claim = SelectedClaim {
                claim_id: s.need_id.clone(),
                canonical_id: cid.clone(),
                source: "selection".to_string(),
            };
            out.push(claim);
        }
    }

    Ok(out)
}
