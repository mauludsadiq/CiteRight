use crate::reasoning::argument_graph::CaseNode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicabilityScore {
    pub canonical_id: String,
    pub case_name: String,
    pub score: f32,
    pub signals: Vec<ApplicabilitySignal>,
    pub verdict: ApplicabilityVerdict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicabilitySignal {
    pub name: String,
    pub value: f32,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApplicabilityVerdict {
    HighlyApplicable,
    ModeratelyApplicable,
    WeaklyApplicable,
    NotApplicable,
}

pub fn score_applicability(node: &CaseNode, claim_text: &str) -> ApplicabilityScore {
    let mut signals = Vec::new();

    // Signal 1: keyword overlap between holdings and claim
    let keyword_score = keyword_overlap_score(&node.holdings, claim_text);
    signals.push(ApplicabilitySignal {
        name: "keyword_overlap".to_string(),
        value: keyword_score,
        explanation: format!("Holding keywords present in claim text: {:.0}%", keyword_score * 100.0),
    });

    // Signal 2: legal concept overlap (constitutional, statutory, procedural markers)
    let concept_score = legal_concept_overlap(&node.holdings, claim_text);
    signals.push(ApplicabilitySignal {
        name: "legal_concept_overlap".to_string(),
        value: concept_score,
        explanation: format!("Shared legal concepts: {:.0}%", concept_score * 100.0),
    });

    // Signal 3: rule extraction match
    let rule_score = rule_match_score(&node.rule_extracted, claim_text);
    signals.push(ApplicabilitySignal {
        name: "rule_match".to_string(),
        value: rule_score,
        explanation: if rule_score > 0.5 {
            "Extracted rule aligns with claim".to_string()
        } else {
            "Extracted rule does not clearly align with claim".to_string()
        },
    });

    // Signal 4: recency (more recent cases weighted higher)
    let recency_score = recency_score(&node.date_filed);
    signals.push(ApplicabilitySignal {
        name: "recency".to_string(),
        value: recency_score,
        explanation: format!("Case recency score: {:.2}", recency_score),
    });

    // Weighted composite score
    let score = (keyword_score * 0.40)
        + (concept_score * 0.30)
        + (rule_score * 0.20)
        + (recency_score * 0.10);

    let verdict = match score {
        s if s >= 0.65 => ApplicabilityVerdict::HighlyApplicable,
        s if s >= 0.40 => ApplicabilityVerdict::ModeratelyApplicable,
        s if s >= 0.20 => ApplicabilityVerdict::WeaklyApplicable,
        _ => ApplicabilityVerdict::NotApplicable,
    };

    ApplicabilityScore {
        canonical_id: node.canonical_id.clone(),
        case_name: node.case_name.clone(),
        score,
        signals,
        verdict,
    }
}

pub fn rank_by_applicability(nodes: &[&CaseNode], claim_text: &str) -> Vec<ApplicabilityScore> {
    let mut scores: Vec<ApplicabilityScore> = nodes.iter()
        .map(|n| score_applicability(n, claim_text))
        .collect();
    scores.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scores
}

fn keyword_overlap_score(holdings: &[String], claim_text: &str) -> f32 {
    if holdings.is_empty() {
        return 0.0;
    }

    let claim_words: std::collections::HashSet<String> = tokenize(claim_text);
    let holding_words: std::collections::HashSet<String> = holdings.iter()
        .flat_map(|h| tokenize(h))
        .collect();

    if holding_words.is_empty() {
        return 0.0;
    }

    let intersection = claim_words.intersection(&holding_words).count();
    intersection as f32 / holding_words.len().max(1) as f32
}

fn legal_concept_overlap(holdings: &[String], claim_text: &str) -> f32 {
    let legal_concepts = [
        "constitutional", "due process", "equal protection", "fundamental right",
        "liberty", "property", "standing", "jurisdiction", "statute", "regulation",
        "amendment", "clause", "right", "freedom", "commerce", "contract",
        "negligence", "liability", "damages", "injunction", "remedy",
        "bankruptcy", "discharge", "creditor", "debtor", "estate",
        "corporation", "fiduciary", "duty", "breach", "tort",
    ];

    let claim_lower = claim_text.to_lowercase();
    let holdings_lower = holdings.join(" ").to_lowercase();

    let claim_concepts: std::collections::HashSet<&str> = legal_concepts.iter()
        .filter(|c| claim_lower.contains(*c))
        .copied()
        .collect();

    let holding_concepts: std::collections::HashSet<&str> = legal_concepts.iter()
        .filter(|c| holdings_lower.contains(*c))
        .copied()
        .collect();

    if holding_concepts.is_empty() {
        return 0.0;
    }

    let intersection = claim_concepts.intersection(&holding_concepts).count();
    intersection as f32 / holding_concepts.len().max(1) as f32
}

fn rule_match_score(rule: &Option<String>, claim_text: &str) -> f32 {
    let rule = match rule {
        Some(r) => r,
        None => return 0.0,
    };

    let rule_words = tokenize(rule);
    let claim_words = tokenize(claim_text);

    if rule_words.is_empty() {
        return 0.0;
    }

    let matches = rule_words.iter()
        .filter(|w| claim_words.contains(*w))
        .count();

    (matches as f32 / rule_words.len() as f32).min(1.0)
}

fn recency_score(date_filed: &Option<String>) -> f32 {
    let year = match date_filed {
        Some(d) => d.split('-').next()
            .and_then(|y| y.parse::<i32>().ok())
            .unwrap_or(1900),
        None => return 0.3,
    };

    // Score from 0.0 (1900) to 1.0 (2026)
    ((year - 1900) as f32 / 126.0).clamp(0.0, 1.0)
}

fn tokenize(text: &str) -> std::collections::HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| w.len() > 3)
        .map(|w| w.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_node(id: &str, name: &str, holding: &str, date: Option<&str>) -> CaseNode {
        CaseNode {
            canonical_id: id.to_string(),
            case_name: name.to_string(),
            date_filed: date.map(|d| d.to_string()),
            holdings: vec![holding.to_string()],
            rule_extracted: Some(holding.to_string()),
            jurisdiction: None,
        }
    }

    #[test]
    fn scores_high_applicability_on_matching_claim() {
        let node = mock_node(
            "cluster:1",
            "Obergefell v. Hodges",
            "We hold that same-sex couples have a fundamental right to marry under due process and equal protection",
            Some("2015-06-26"),
        );
        let claim = "The constitutional due process and equal protection clauses guarantee fundamental rights to all persons.";
        let score = score_applicability(&node, claim);
        assert!(score.score > 0.3, "Expected moderate+ score, got {}", score.score);
    }

    #[test]
    fn scores_low_applicability_on_unrelated_claim() {
        let node = mock_node(
            "cluster:2",
            "Conti v. Perdue Bioenergy",
            "The Fourth Circuit held that commodity forward agreements need not be traded on exchange",
            Some("2015-09-29"),
        );
        let claim = "Constitutional due process guarantees fundamental liberty rights.";
        let score = score_applicability(&node, claim);
        assert!(score.score < 0.5, "Expected low score, got {}", score.score);
    }

    #[test]
    fn ranks_multiple_cases_by_applicability() {
        let n1 = mock_node("cluster:1", "Case A", "fundamental right liberty due process constitutional", Some("2020-01-01"));
        let n2 = mock_node("cluster:2", "Case B", "commodity forward agreement exchange physical delivery", Some("2015-01-01"));
        let claim = "The constitutional liberty interest is protected by due process.";
        let ranked = rank_by_applicability(&[&n1, &n2], claim);
        assert_eq!(ranked[0].canonical_id, "cluster:1");
    }

    #[test]
    fn returns_verdict_for_each_score() {
        let node = mock_node("cluster:1", "Test", "We hold X applies", Some("2023-01-01"));
        let score = score_applicability(&node, "X applies here");
        assert!(matches!(
            score.verdict,
            ApplicabilityVerdict::HighlyApplicable
                | ApplicabilityVerdict::ModeratelyApplicable
                | ApplicabilityVerdict::WeaklyApplicable
                | ApplicabilityVerdict::NotApplicable
        ));
    }
}
