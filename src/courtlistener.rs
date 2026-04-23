use crate::hash::sha256_bytes;
use crate::models::{CanonicalCluster, ClusterCitation, LookupRecord};
use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

pub trait CitationLookup {
    fn lookup_text(&self, text: &str) -> Result<Vec<LookupRecord>>;
}

pub struct CourtListenerClient {
    client: Client,
    token: Option<String>,
    endpoint: String,
}

impl CourtListenerClient {
    pub fn new(token: Option<String>, endpoint: Option<String>) -> Result<Self> {
        Ok(Self {
            client: Client::builder().timeout(Duration::from_secs(30)).user_agent("CiteRight/0.1").build()?,
            token,
            endpoint: endpoint.unwrap_or_else(|| "https://www.courtlistener.com/api/rest/v4/citation-lookup/".to_string()),
        })
    }
}

impl CitationLookup for CourtListenerClient {
    fn lookup_text(&self, text: &str) -> Result<Vec<LookupRecord>> {
        let mut req = self.client.post(&self.endpoint).form(&[("text", text)]);
        if let Some(token) = &self.token {
            req = req.header("Authorization", format!("Token {token}"));
        }
        let request_digest = sha256_bytes(format!("POST:{}:{}", self.endpoint, text).as_bytes());
        let resp = req.send().context("send CourtListener citation lookup request")?;
        let http_status = resp.status().as_u16();
        let body = resp.text().context("read CourtListener response body")?;
        let raw_response_digest = sha256_bytes(body.as_bytes());
        if !(200..300).contains(&http_status) {
            anyhow::bail!("CourtListener HTTP {http_status}: {body}");
        }
        let parsed: Vec<ClCitation> = serde_json::from_str(&body).context("parse CourtListener citation lookup JSON")?;
        Ok(parsed.into_iter().map(|c| c.into_record(request_digest.clone(), Some(http_status), raw_response_digest.clone())).collect())
    }
}

pub struct FixtureLookup {
    records: BTreeMap<String, LookupRecord>,
}

impl FixtureLookup {
    pub fn from_file(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path).with_context(|| format!("read fixture {}", path.display()))?;
        let records_vec: Vec<LookupRecord> = serde_json::from_slice(&bytes).context("parse fixture lookup records")?;
        let records = records_vec.into_iter().map(|r| (r.raw_citation.clone(), r)).collect();
        Ok(Self { records })
    }
}

impl CitationLookup for FixtureLookup {
    fn lookup_text(&self, text: &str) -> Result<Vec<LookupRecord>> {
        let mut out = Vec::new();
        for (citation, record) in &self.records {
            if text.contains(citation) {
                out.push(record.clone());
            }
        }
        Ok(out)
    }
}

#[derive(Debug, Deserialize)]
struct ClCitation {
    citation: String,
    #[serde(default)]
    normalized_citations: Vec<String>,
    #[serde(default)]
    status: Option<u16>,
    #[serde(default)]
    error_message: String,
    #[serde(default)]
    clusters: Vec<Value>,
}

impl ClCitation {
    fn into_record(self, request_digest: String, http_status: Option<u16>, raw_response_digest: String) -> LookupRecord {
        LookupRecord {
            source: "courtlistener:v4:citation-lookup".to_string(),
            request_digest,
            http_status,
            api_status: self.status,
            raw_citation: self.citation,
            normalized_citations: self.normalized_citations,
            clusters: self.clusters.into_iter().map(cluster_from_value).collect(),
            error_message: self.error_message,
            raw_response_digest,
        }
    }
}

fn cluster_from_value(v: Value) -> CanonicalCluster {
    let id = v.get("id").and_then(|x| x.as_i64());
    let case_name = v.get("case_name").or_else(|| v.get("case_name_full")).and_then(|x| x.as_str()).map(|s| s.to_string());
    let absolute_url = v.get("absolute_url").and_then(|x| x.as_str()).map(|s| s.to_string());
    let date_filed = v.get("date_filed").and_then(|x| x.as_str()).map(|s| s.to_string());
    let citations = v.get("citations").and_then(|x| x.as_array()).map(|arr| arr.iter().map(citation_from_value).collect()).unwrap_or_default();
    CanonicalCluster { id, case_name, absolute_url, date_filed, citations }
}

fn citation_from_value(v: &Value) -> ClusterCitation {
    ClusterCitation {
        volume: v.get("volume").and_then(|x| x.as_i64()),
        reporter: v.get("reporter").and_then(|x| x.as_str()).map(|s| s.to_string()),
        page: v.get("page").and_then(|x| x.as_str()).map(|s| s.to_string()),
        citation_type: v.get("type").and_then(|x| x.as_i64()),
    }
}
