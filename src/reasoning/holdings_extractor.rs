use crate::artifact::CaseArtifact;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldingNode {
    pub canonical_id: String,
    pub case_name: String,
    pub holdings: Vec<ExtractedHolding>,
    pub opinion_url: Option<String>,
    pub extraction_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedHolding {
    pub text: String,
    pub trigger: String,
    pub confidence: HoldingConfidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HoldingConfidence {
    High,   // "We hold that"
    Medium, // "we conclude", "we affirm", "we reverse"
    Low,    // "the Court holds", "held that"
}

static HIGH_TRIGGERS: &[&str] = &[
    "We hold that",
    "We hold ",
    "we hold that",
    "we hold ",
];

static MEDIUM_TRIGGERS: &[&str] = &[
    "We conclude that",
    "we conclude that",
    "We affirm ",
    "we affirm ",
    "We reverse ",
    "we reverse ",
    "We vacate ",
    "we vacate ",
];

static LOW_TRIGGERS: &[&str] = &[
    "the Court holds",
    "The Court holds",
    "held that",
    "Court held",
    "we find that",
    "We find that",
];

pub fn extract_holdings_from_text(
    artifact: &CaseArtifact,
    opinion_text: &str,
) -> HoldingNode {
    let plain = strip_html(opinion_text);
    let mut holdings = Vec::new();

    for trigger in HIGH_TRIGGERS {
        extract_sentences(&plain, trigger, HoldingConfidence::High, &mut holdings);
    }
    for trigger in MEDIUM_TRIGGERS {
        extract_sentences(&plain, trigger, HoldingConfidence::Medium, &mut holdings);
    }
    for trigger in LOW_TRIGGERS {
        extract_sentences(&plain, trigger, HoldingConfidence::Low, &mut holdings);
    }

    // Dedup by text
    holdings.dedup_by(|a, b| a.text == b.text);

    HoldingNode {
        canonical_id: artifact.canonical_id.clone(),
        case_name: artifact.case_name.clone(),
        holdings,
        opinion_url: artifact.absolute_url.clone(),
        extraction_method: "pattern_v1".to_string(),
    }
}

fn extract_sentences(
    text: &str,
    trigger: &str,
    confidence: HoldingConfidence,
    out: &mut Vec<ExtractedHolding>,
) {
    let mut start = 0;
    while let Some(pos) = text[start..].find(trigger) {
        let abs_pos = start + pos;

        // Find sentence start (look back for period or start of text)
        let sentence_start = text[..abs_pos]
            .rfind(|c| c == '.' || c == '\n')
            .map(|p| p + 1)
            .unwrap_or(0);

        // Find sentence end (look forward for period)
        let sentence_end = text[abs_pos..]
            .find('.')
            .map(|p| abs_pos + p + 1)
            .unwrap_or(text.len().min(abs_pos + 300));

        let sentence = text[sentence_start..sentence_end].trim().to_string();

        if sentence.len() > 20 && sentence.len() < 800 {
            out.push(ExtractedHolding {
                text: sentence,
                trigger: trigger.to_string(),
                confidence: confidence.clone(),
            });
        }

        start = abs_pos + trigger.len();
        if start >= text.len() {
            break;
        }
    }
}

fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    // Normalize whitespace
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::VerificationStatus;

    fn mock_artifact() -> CaseArtifact {
        CaseArtifact {
            artifact_id: "sha256:test".to_string(),
            canonical_id: "cluster:2812209".to_string(),
            case_name: "Obergefell v. Hodges".to_string(),
            citations: vec!["576 U.S. 644".to_string()],
            date_filed: Some("2015-06-26".to_string()),
            absolute_url: Some("/opinion/2812209/obergefell-v-hodges/".to_string()),
            source: "test".to_string(),
            source_digest: "sha256:test".to_string(),
            verification_status: VerificationStatus::Verified,
        }
    }

    #[test]
    fn extracts_holding_from_opinion_text() {
        let artifact = mock_artifact();
        let text = "<p>The Court considered the matter carefully. We hold that same-sex couples may exercise the fundamental right to marry. The decision was unanimous.</p>";
        let node = extract_holdings_from_text(&artifact, text);
        assert_eq!(node.canonical_id, "cluster:2812209");
        assert!(!node.holdings.is_empty());
        assert!(node.holdings[0].text.contains("We hold that"));
    }

    #[test]
    fn returns_empty_holdings_when_none_found() {
        let artifact = mock_artifact();
        let text = "<p>This opinion contains no holding language.</p>";
        let node = extract_holdings_from_text(&artifact, text);
        assert!(node.holdings.is_empty());
    }
}
