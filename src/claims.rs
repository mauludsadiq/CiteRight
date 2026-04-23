use crate::hash::sha256_json;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClaimKind {
    NeedsAuthority,
    NamedAuthority,
    HoldingClaim,
    RuleStatement,
    FactualAssertion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalClaim {
    pub claim_id: String,
    pub kind: ClaimKind,
    pub text: String,
    pub start_index: usize,
    pub end_index: usize,
    pub confidence_basis: String,
}

pub fn extract_legal_claims(text: &str) -> Result<Vec<LegalClaim>> {
    let mut claims = Vec::new();

    for sentence in split_sentences_with_offsets(text) {
        let lower = sentence.text.to_lowercase();

        let kind = if lower.contains(" v. ") || lower.contains(" in re ") {
            Some(ClaimKind::NamedAuthority)
        } else if lower.contains("held that") || lower.contains("the court held") || lower.contains("confirmed that") {
            Some(ClaimKind::HoldingClaim)
        } else if lower.contains("must ") || lower.contains("may not ") || lower.contains("shall ") || lower.contains("requires ") {
            Some(ClaimKind::RuleStatement)
        } else if lower.contains("authority") || lower.contains("citation") || lower.contains("case law") {
            Some(ClaimKind::NeedsAuthority)
        } else {
            None
        };

        if let Some(kind) = kind {
            let mut claim = LegalClaim {
                claim_id: "pending".to_string(),
                kind,
                text: sentence.text.trim().to_string(),
                start_index: sentence.start,
                end_index: sentence.end,
                confidence_basis: "deterministic lexical rule v0".to_string(),
            };
            claim.claim_id = sha256_json(&claim)?;
            claims.push(claim);
        }
    }

    Ok(claims)
}

#[derive(Debug)]
struct SentenceSpan {
    text: String,
    start: usize,
    end: usize,
}

fn split_sentences_with_offsets(text: &str) -> Vec<SentenceSpan> {
    let mut out = Vec::new();
    let mut start = 0usize;

    for (idx, ch) in text.char_indices() {
        if is_sentence_boundary(text, idx, ch) {
            let end = idx + ch.len_utf8();
            let slice = &text[start..end];
            if !slice.trim().is_empty() {
                out.push(SentenceSpan {
                    text: slice.to_string(),
                    start,
                    end,
                });
            }
            start = end;
            while start < text.len() {
                let next = text[start..].chars().next();
                if matches!(next, Some(c) if c.is_whitespace()) {
                    start += next.unwrap().len_utf8();
                } else {
                    break;
                }
            }
        }
    }

    if start < text.len() {
        let slice = &text[start..];
        if !slice.trim().is_empty() {
            out.push(SentenceSpan {
                text: slice.to_string(),
                start,
                end: text.len(),
            });
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_rule_and_holding_claims() {
        let text = "The Court held that states may not exclude a protected class. This sentence is background.";
        let claims = extract_legal_claims(text).unwrap();
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].kind, ClaimKind::HoldingClaim);
    }
}

fn is_sentence_boundary(text: &str, idx: usize, ch: char) -> bool {
    if matches!(ch, '!' | '?') {
        return true;
    }

    if ch != '.' {
        return false;
    }

    let before = &text[..idx.min(text.len())];
    let after = &text[idx..];

    let tail: String = before.chars().rev().take(12).collect::<String>().chars().rev().collect();
    let head: String = after.chars().take(12).collect();

    let window = format!("{}{}", tail, head);

    let protected = [
        "U.S.",
        "S.Ct.",
        "L.Ed.",
        "F.2d",
        "F.3d",
        "F.Supp.",
        "B.R.",
        "N.E.",
        "N.W.",
        "S.E.",
        "S.W.",
        "A.2d",
        "A.3d",
    ];

    !protected.iter().any(|abbr| window.contains(abbr))
}
