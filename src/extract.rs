use crate::hash::sha256_bytes;
use crate::models::CitationClaim;
use regex::Regex;

pub fn extract_citations(text: &str) -> anyhow::Result<Vec<CitationClaim>> {
    let re = Regex::new(r"(?x)\b(?P<vol>\d{1,4})\s+(?P<rep>(?:[A-Z][A-Za-z.]*|B\.R\.|F\.\s?Supp\.?\s?\d?d?|F\.\s?\d?d?|U\.S\.|S\.\s?Ct\.|L\.\s?Ed\.\s?\d?d?)(?:\s?[A-Za-z.]+)*)\s+(?P<page>\d{1,5})\b")?;
    let mut out = Vec::new();
    for m in re.find_iter(text) {
        let raw = m.as_str().trim().to_string();
        let start = m.start();
        let end = m.end();
        let context_before = safe_window(text, start.saturating_sub(160), start);
        let context_after = safe_window(text, end, (end + 160).min(text.len()));
        let quoted_text = nearest_quote(&context_before, &context_after);
        let normalized_guess = normalize_spaces(&raw);
        let id_material = format!("{}:{}:{}", start, end, raw);
        out.push(CitationClaim {
            claim_id: sha256_bytes(id_material.as_bytes()),
            raw,
            normalized_guess,
            start_index: start,
            end_index: end,
            context_before,
            context_after,
            quoted_text,
        });
    }
    Ok(dedup_claims(out))
}

fn normalize_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn safe_window(text: &str, start: usize, end: usize) -> String {
    text.get(start..end).unwrap_or("").to_string()
}

fn nearest_quote(before: &str, after: &str) -> Option<String> {
    let joined = format!("{}{}", before, after);
    let quote_re = Regex::new(r#"[“\"]([^”\"]{8,240})[”\"]"#).ok()?;
    quote_re
        .captures(&joined)
        .and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
}

fn dedup_claims(claims: Vec<CitationClaim>) -> Vec<CitationClaim> {
    let mut seen = std::collections::BTreeSet::new();
    claims
        .into_iter()
        .filter(|c| seen.insert((c.start_index, c.end_index, c.raw.clone())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracts_basic_citations() {
        let c = extract_citations("See Obergefell v. Hodges, 576 U.S. 644 and 1 U.S. 200.").unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].raw, "576 U.S. 644");
    }
}
