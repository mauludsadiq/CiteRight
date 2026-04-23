use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunReceipt {
    pub run_id: String,
    pub tool: String,
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub input_path: String,
    pub input_digest: String,
    pub policy: GatePolicy,
    pub counts: RunCounts,
    pub artifacts: OutputArtifacts,
    pub verified: bool,
    pub receipt_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatePolicy {
    pub block_unverified: bool,
    pub heal_unique_normalized: bool,
    pub require_single_cluster: bool,
    pub require_quote_match_when_present: bool,
}

impl Default for GatePolicy {
    fn default() -> Self {
        Self {
            block_unverified: true,
            heal_unique_normalized: true,
            require_single_cluster: true,
            require_quote_match_when_present: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunCounts {
    pub extracted_citations: usize,
    pub verified: usize,
    pub healed: usize,
    pub ambiguous: usize,
    pub blocked: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputArtifacts {
    pub verified_brief_md: String,
    pub failed_claims_json: String,
    pub citations_json: String,
    pub receipt_chain_json: String,
    pub run_receipt_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationClaim {
    pub claim_id: String,
    pub raw: String,
    pub normalized_guess: String,
    pub start_index: usize,
    pub end_index: usize,
    pub context_before: String,
    pub context_after: String,
    pub quoted_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRecord {
    pub claim: CitationClaim,
    pub lookup: LookupRecord,
    pub status: ClaimStatus,
    pub reason: String,
    pub action: ClaimAction,
    pub receipt_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClaimStatus {
    Verified,
    Healed,
    Ambiguous,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClaimAction {
    Keep,
    ReplaceWithNormalized,
    BlockOrRedact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupRecord {
    pub source: String,
    pub request_digest: String,
    pub http_status: Option<u16>,
    pub api_status: Option<u16>,
    pub raw_citation: String,
    pub normalized_citations: Vec<String>,
    pub clusters: Vec<CanonicalCluster>,
    pub error_message: String,
    pub raw_response_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalCluster {
    pub id: Option<i64>,
    pub case_name: Option<String>,
    pub absolute_url: Option<String>,
    pub date_filed: Option<String>,
    pub citations: Vec<ClusterCitation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterCitation {
    pub volume: Option<i64>,
    pub reporter: Option<String>,
    pub page: Option<String>,
    #[serde(rename = "type")]
    pub citation_type: Option<i64>,
}
