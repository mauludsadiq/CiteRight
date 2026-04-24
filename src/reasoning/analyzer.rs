use crate::artifact::CaseArtifact;
use crate::reasoning::holdings_extractor::{extract_holdings_from_text, HoldingNode};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub canonical_id: String,
    pub case_name: String,
    pub holding_node: HoldingNode,
    pub claim_text: String,
    pub assessment: ClaimAssessment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimAssessment {
    pub supported: SupportLevel,
    pub explanation: String,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SupportLevel {
    Supported,
    PartiallySupported,
    Unsupported,
    Indeterminate,
}

pub fn fetch_opinion_and_analyze(
    artifact: &CaseArtifact,
    claim_text: &str,
    token: &str,
) -> Result<AnalysisResult> {
    let cluster_id = artifact.canonical_id
        .strip_prefix("cluster:")
        .ok_or_else(|| anyhow!("not a cluster ID: {}", artifact.canonical_id))?;

    // Fetch majority opinion from CourtListener
    let url = format!(
        "https://www.courtlistener.com/api/rest/v4/opinions/?cluster={}",
        cluster_id
    );

    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&url)
        .header("Authorization", format!("Token {}", token))
        .send()?;

    if !resp.status().is_success() {
        return Err(anyhow!("CourtListener opinion fetch failed: {}", resp.status()));
    }

    let json: serde_json::Value = resp.json()?;
    let results = json["results"].as_array()
        .ok_or_else(|| anyhow!("no results in opinion response"))?;

    // Find the opinion text - try html_with_citations, then plain_text
    let opinion_text = results.iter()
        .find_map(|r| {
            r["html_with_citations"].as_str()
                .filter(|s| !s.is_empty())
                .or_else(|| r["plain_text"].as_str().filter(|s| !s.is_empty()))
        })
        .ok_or_else(|| anyhow!("no opinion text found for cluster {}", cluster_id))?;

    // Extract holdings
    let holding_node = extract_holdings_from_text(artifact, opinion_text);

    // Build holdings summary for Claude
    let holdings_text = if holding_node.holdings.is_empty() {
        "No explicit holding language found in majority opinion.".to_string()
    } else {
        holding_node.holdings.iter()
            .take(5)
            .map(|h| format!("[{}] {}", format!("{:?}", h.confidence), h.text))
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Call Claude API for assessment
    let assessment = call_claude_for_assessment(
        &artifact.case_name,
        &holdings_text,
        claim_text,
    )?;

    Ok(AnalysisResult {
        canonical_id: artifact.canonical_id.clone(),
        case_name: artifact.case_name.clone(),
        holding_node,
        claim_text: claim_text.to_string(),
        assessment,
    })
}

fn call_claude_for_assessment(
    case_name: &str,
    holdings_text: &str,
    claim_text: &str,
) -> Result<ClaimAssessment> {
    let prompt = [
        "You are a legal citation analyst. Assess whether the following legal claim correctly characterizes the holding of the cited case.",
        "",
        &format!("Case: {}", case_name),
        "",
        "Extracted holdings from majority opinion:",
        holdings_text,
        "",
        "Claim made in brief:",
        claim_text,
        "",
        "Respond in JSON only, no markdown, no backticks, with this exact structure:",
        r#"{"supported": "Supported or PartiallySupported or Unsupported or Indeterminate", "explanation": "one or two sentence explanation", "confidence": "High or Medium or Low"}"#,
    ].join("\n");

    let body = serde_json::json!({
        "model": "gpt-4o-mini",
        "max_tokens": 512,
        "messages": [{"role": "user", "content": prompt}]
    });

    let client = reqwest::blocking::Client::new();
    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", std::env::var("OPENAI_API_KEY").unwrap_or_default()))
        .json(&body)
        .send()?;

    if !resp.status().is_success() {
        return Ok(ClaimAssessment {
            supported: SupportLevel::Indeterminate,
            explanation: format!("OpenAI API unavailable: {}", resp.status()),
            confidence: "Low".to_string(),
        });
    }

    let json: serde_json::Value = resp.json()?;
    let text = json["choices"][0]["message"]["content"].as_str().unwrap_or("{}");
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap_or(serde_json::json!({}));

    let supported = match parsed["supported"].as_str().unwrap_or("Indeterminate") {
        "Supported" => SupportLevel::Supported,
        "PartiallySupported" => SupportLevel::PartiallySupported,
        "Unsupported" => SupportLevel::Unsupported,
        _ => SupportLevel::Indeterminate,
    };

    Ok(ClaimAssessment {
        supported,
        explanation: parsed["explanation"].as_str().unwrap_or("No explanation provided").to_string(),
        confidence: parsed["confidence"].as_str().unwrap_or("Low").to_string(),
    })
}
