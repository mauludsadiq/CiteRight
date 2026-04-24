use crate::reasoning::holdings_extractor::HoldingNode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseNode {
    pub canonical_id: String,
    pub case_name: String,
    pub date_filed: Option<String>,
    pub holdings: Vec<String>,
    pub rule_extracted: Option<String>,
    pub jurisdiction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EdgeType {
    Supports,
    Contradicts,
    Distinguishes,
    Cites,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from_id: String,
    pub to_id: String,
    pub edge_type: EdgeType,
    pub confidence: f32,
    pub explanation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentClaim {
    pub claim_id: String,
    pub text: String,
    pub cited_case_ids: Vec<String>,
    pub assessment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentGraph {
    pub nodes: HashMap<String, CaseNode>,
    pub edges: Vec<Edge>,
    pub claims: Vec<ArgumentClaim>,
    pub graph_digest: String,
}

impl ArgumentGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            claims: Vec::new(),
            graph_digest: String::new(),
        }
    }

    pub fn add_node_from_holding(&mut self, holding: &HoldingNode) {
        let holdings_text: Vec<String> = holding.holdings
            .iter()
            .map(|h| h.text.clone())
            .collect();

        let rule = holdings_text.first().cloned();

        self.nodes.insert(holding.canonical_id.clone(), CaseNode {
            canonical_id: holding.canonical_id.clone(),
            case_name: holding.case_name.clone(),
            date_filed: None,
            holdings: holdings_text,
            rule_extracted: rule,
            jurisdiction: None,
        });
    }

    pub fn add_edge(&mut self, from_id: &str, to_id: &str, edge_type: EdgeType, confidence: f32, explanation: Option<String>) {
        self.edges.push(Edge {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
            edge_type,
            confidence,
            explanation,
        });
    }

    pub fn add_claim(&mut self, claim_id: &str, text: &str, cited_case_ids: Vec<String>) {
        self.claims.push(ArgumentClaim {
            claim_id: claim_id.to_string(),
            text: text.to_string(),
            cited_case_ids,
            assessment: None,
        });
    }

    pub fn set_claim_assessment(&mut self, claim_id: &str, assessment: &str) {
        if let Some(claim) = self.claims.iter_mut().find(|c| c.claim_id == claim_id) {
            claim.assessment = Some(assessment.to_string());
        }
    }

    pub fn cases_supporting_claim(&self, claim_id: &str) -> Vec<&CaseNode> {
        let claim = match self.claims.iter().find(|c| c.claim_id == claim_id) {
            Some(c) => c,
            None => return vec![],
        };

        claim.cited_case_ids.iter()
            .filter_map(|id| self.nodes.get(id))
            .collect()
    }

    pub fn conflicting_cases(&self, canonical_id: &str) -> Vec<&CaseNode> {
        self.edges.iter()
            .filter(|e| e.from_id == canonical_id && e.edge_type == EdgeType::Contradicts)
            .filter_map(|e| self.nodes.get(&e.to_id))
            .collect()
    }

    pub fn finalize(&mut self) {
        use sha2::{Sha256, Digest};
        let json = serde_json::to_string(&(&self.nodes, &self.edges, &self.claims))
            .unwrap_or_default();
        let hash = Sha256::digest(json.as_bytes());
        self.graph_digest = format!("sha256:{}", hex::encode(hash));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reasoning::holdings_extractor::{HoldingNode, ExtractedHolding, HoldingConfidence};

    fn mock_holding(id: &str, name: &str, holding_text: &str) -> HoldingNode {
        HoldingNode {
            canonical_id: id.to_string(),
            case_name: name.to_string(),
            holdings: vec![ExtractedHolding {
                text: holding_text.to_string(),
                trigger: "We hold that".to_string(),
                confidence: HoldingConfidence::High,
            }],
            opinion_url: None,
            extraction_method: "pattern_v1".to_string(),
        }
    }

    #[test]
    fn builds_graph_from_holding_nodes() {
        let mut graph = ArgumentGraph::new();
        let h = mock_holding("cluster:1", "Test v. Case", "We hold that X applies.");
        graph.add_node_from_holding(&h);
        assert!(graph.nodes.contains_key("cluster:1"));
        assert_eq!(graph.nodes["cluster:1"].case_name, "Test v. Case");
    }

    #[test]
    fn adds_edges_between_cases() {
        let mut graph = ArgumentGraph::new();
        graph.add_edge("cluster:1", "cluster:2", EdgeType::Supports, 0.9, Some("directly cited".to_string()));
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].edge_type, EdgeType::Supports);
    }

    #[test]
    fn tracks_claims_and_cited_cases() {
        let mut graph = ArgumentGraph::new();
        let h = mock_holding("cluster:1", "Test v. Case", "We hold that X applies.");
        graph.add_node_from_holding(&h);
        graph.add_claim("claim:1", "X applies to these facts.", vec!["cluster:1".to_string()]);
        let supporting = graph.cases_supporting_claim("claim:1");
        assert_eq!(supporting.len(), 1);
        assert_eq!(supporting[0].case_name, "Test v. Case");
    }

    #[test]
    fn finalizes_with_digest() {
        let mut graph = ArgumentGraph::new();
        graph.finalize();
        assert!(graph.graph_digest.starts_with("sha256:"));
    }

    #[test]
    fn finds_conflicting_cases() {
        let mut graph = ArgumentGraph::new();
        let h1 = mock_holding("cluster:1", "Case A", "We hold X.");
        let h2 = mock_holding("cluster:2", "Case B", "We hold not X.");
        graph.add_node_from_holding(&h1);
        graph.add_node_from_holding(&h2);
        graph.add_edge("cluster:1", "cluster:2", EdgeType::Contradicts, 0.8, None);
        let conflicts = graph.conflicting_cases("cluster:1");
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].canonical_id, "cluster:2");
    }
}
