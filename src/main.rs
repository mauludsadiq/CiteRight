mod claims;
mod courtlistener;
mod document;
mod emit;
mod extract;
mod hash;
mod models;
mod planner;
mod candidates;
mod artifact;
mod audit;
mod verify_selected;
mod resolver;
mod selector;
mod verify;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use courtlistener::{CitationLookup, CourtListenerClient, FixtureLookup};
use emit::{emit_verified_markdown, write_json};
use hash::{sha256_bytes, sha256_json};
use models::*;
use std::path::PathBuf;
use verify::{counts, verify_claims};

#[derive(Parser, Debug)]
#[command(name = "citeright", version, about = "Deterministic legal citation proof gate")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Verify {
        input: PathBuf,
        #[arg(long, default_value = "out/cite_right_run")]
        out: PathBuf,
        #[arg(long, env = "COURTLISTENER_TOKEN")]
        courtlistener_token: Option<String>,
        #[arg(long)]
        courtlistener_endpoint: Option<String>,
        #[arg(long)]
        offline_fixtures: Option<PathBuf>,
        #[arg(long, default_value_t = true)]
        block_unverified: bool,
        #[arg(long, default_value_t = true)]
        heal_unique_normalized: bool,
    },
    Extract { input: PathBuf },
    Claims { input: PathBuf },
    Plan { input: PathBuf },
    Candidates { input: PathBuf },
    Resolve { input: PathBuf, #[arg(long)] offline_fixtures: PathBuf },
    Select { input: PathBuf, #[arg(long)] offline_fixtures: PathBuf },
    Audit { input: PathBuf, #[arg(long)] offline_fixtures: PathBuf, #[arg(long, default_value = "out/ai_audit")] out: PathBuf },
    VerifyAi { input: PathBuf, #[arg(long)] offline_fixtures: PathBuf, #[arg(long, default_value = "out/ai_verify")] out: PathBuf },
    Artifacts { input: PathBuf, #[arg(long)] offline_fixtures: PathBuf },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Verify { input, out, courtlistener_token, courtlistener_endpoint, offline_fixtures, block_unverified, heal_unique_normalized } => {
            run_verify(input, out, courtlistener_token, courtlistener_endpoint, offline_fixtures, block_unverified, heal_unique_normalized)
        }
        Commands::Extract { input } => {
            let text = document::read_document(&input)?;
            let claims = extract::extract_citations(&text)?;
            println!("{}", serde_json::to_string_pretty(&claims)?);
            Ok(())
        }
        Commands::Claims { input } => {
            let text = document::read_document(&input)?;
            let claims = claims::extract_legal_claims(&text)?;
            println!("{}", serde_json::to_string_pretty(&claims)?);
            Ok(())
        }
        Commands::Plan { input } => {
            let text = document::read_document(&input)?;
            let claims = claims::extract_legal_claims(&text)?;
            let needs = planner::plan_citation_needs(&claims)?;
            println!("{}", serde_json::to_string_pretty(&needs)?);
            Ok(())
        }
        Commands::Candidates { input } => {
            let text = document::read_document(&input)?;
            let claims = claims::extract_legal_claims(&text)?;
            let needs = planner::plan_citation_needs(&claims)?;
            let candidates = candidates::generate_candidates(&needs)?;
            println!("{}", serde_json::to_string_pretty(&candidates)?);
            Ok(())
        }
        Commands::Resolve { input, offline_fixtures } => {
            let text = document::read_document(&input)?;
            let claims = claims::extract_legal_claims(&text)?;
            let needs = planner::plan_citation_needs(&claims)?;
            let candidates = candidates::generate_candidates(&needs)?;
            let resolutions = resolver::resolve_candidates_with_fixtures(&candidates, &offline_fixtures)?;
            println!("{}", serde_json::to_string_pretty(&resolutions)?);
            Ok(())
        }
        Commands::Select { input, offline_fixtures } => {
            let text = document::read_document(&input)?;
            let claims = claims::extract_legal_claims(&text)?;
            let needs = planner::plan_citation_needs(&claims)?;
            let candidates = candidates::generate_candidates(&needs)?;
            let resolutions = resolver::resolve_candidates_with_fixtures(&candidates, &offline_fixtures)?;
            let selections = selector::select_best_candidates(&needs, &candidates, &resolutions)?;
            println!("{}", serde_json::to_string_pretty(&selections)?);
            Ok(())
        }
        Commands::Audit { input, offline_fixtures, out } => {
            let text = document::read_document(&input)?;
            let claims = claims::extract_legal_claims(&text)?;
            let needs = planner::plan_citation_needs(&claims)?;
            let candidates = candidates::generate_candidates(&needs)?;
            let resolutions = resolver::resolve_candidates_with_fixtures(&candidates, &offline_fixtures)?;
            let selections = selector::select_best_candidates(&needs, &candidates, &resolutions)?;
            let receipt = audit::write_ai_audit(&out, &input, &claims, &needs, &candidates, &resolutions, &selections)?;
            println!("{}", serde_json::to_string_pretty(&receipt)?);
            Ok(())
        }
        Commands::VerifyAi { input, offline_fixtures, out: _ } => {
            let text = document::read_document(&input)?;
            let claims = claims::extract_legal_claims(&text)?;
            let needs = planner::plan_citation_needs(&claims)?;
            let candidates = candidates::generate_candidates(&needs)?;
            let resolutions = resolver::resolve_candidates_with_fixtures(&candidates, &offline_fixtures)?;
            let selections = selector::select_best_candidates(&needs, &candidates, &resolutions)?;
            let selected_claims = verify_selected::selected_to_claims(&selections)?;
            println!("{}", serde_json::to_string_pretty(&selected_claims)?);
            Ok(())
        }
        Commands::Artifacts { input, offline_fixtures } => {
            let text = document::read_document(&input)?;
            let lookup = courtlistener::FixtureLookup::from_file(&offline_fixtures)?;
            let lookups = lookup.lookup_text(&text)?;
            let artifacts = artifact::artifacts_from_lookup_results(&lookups)?;
            println!("{}", serde_json::to_string_pretty(&artifacts)?);
            Ok(())
        }
    }
}

fn run_verify(input: PathBuf, out: PathBuf, token: Option<String>, endpoint: Option<String>, fixtures: Option<PathBuf>, block_unverified: bool, heal_unique_normalized: bool) -> Result<()> {
    std::fs::create_dir_all(&out).with_context(|| format!("create output dir {}", out.display()))?;
    let bytes = std::fs::read(&input).with_context(|| format!("read input bytes {}", input.display()))?;
    let input_digest = sha256_bytes(&bytes);
    let text = document::read_document(&input)?;
    let claims = extract::extract_citations(&text)?;

    let lookups = if let Some(fixture_path) = fixtures {
        FixtureLookup::from_file(&fixture_path)?.lookup_text(&text)?
    } else {
        CourtListenerClient::new(token, endpoint)?.lookup_text(&text)?
    };

    let policy = GatePolicy { block_unverified, heal_unique_normalized, ..GatePolicy::default() };
    let records = verify_claims(claims, lookups, &policy)?;
    let counts = counts(&records);
    let verified = counts.blocked == 0 && counts.ambiguous == 0;

    let verified_md = emit_verified_markdown(&text, &records);
    let failed: Vec<_> = records.iter().filter(|r| matches!(r.status, ClaimStatus::Blocked | ClaimStatus::Ambiguous)).cloned().collect();
    let receipt_chain: Vec<_> = records.iter().map(|r| serde_json::json!({
        "claim_id": r.claim.claim_id,
        "raw": r.claim.raw,
        "status": r.status,
        "action": r.action,
        "receipt_digest": r.receipt_digest
    })).collect::<Vec<_>>();

    let artifacts = OutputArtifacts {
        verified_brief_md: out.join("verified_brief.md").display().to_string(),
        failed_claims_json: out.join("failed_claims.json").display().to_string(),
        citations_json: out.join("citations.json").display().to_string(),
        receipt_chain_json: out.join("receipt_chain.json").display().to_string(),
        run_receipt_json: out.join("run_receipt.json").display().to_string(),
    };

    std::fs::write(out.join("verified_brief.md"), verified_md)?;
    write_json(&out.join("failed_claims.json"), &failed)?;
    write_json(&out.join("citations.json"), &records)?;
    write_json(&out.join("receipt_chain.json"), &receipt_chain)?;

    let mut run = RunReceipt {
        run_id: "pending".to_string(),
        tool: "Cite Right".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        created_at: Utc::now(),
        input_path: input.display().to_string(),
        input_digest,
        policy,
        counts,
        artifacts,
        verified,
        receipt_digest: "pending".to_string(),
    };
    run.run_id = sha256_json(&records)?;
    run.receipt_digest = sha256_json(&run)?;
    write_json(&out.join("run_receipt.json"), &run)?;
    println!("cite_right_run_id={}", run.run_id);
    println!("cite_right_receipt={}", run.receipt_digest);
    println!("verified={}", run.verified);
    Ok(())
}
