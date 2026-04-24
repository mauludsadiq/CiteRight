# CiteRight

CiteRight is a Rust command-line tool that verifies legal citations in AI-drafted documents before filing. It is a pre-filing proof gate.

It answers one question per citation: does this case actually exist in a canonical legal source?

If it does not, the citation is flagged UNVERIFIED and the pipeline exits non-zero before the document can be filed. This is the class of error that caused the Sullivan and Cromwell AI hallucination incident.

## How It Works

1. Extract all citations from the input document
2. Look up each citation against CourtListener (live API or offline fixture)
3. Produce an explicit VERIFIED or UNVERIFIED artifact for every citation
4. Generate an attorney-signed audit receipt bound to the document SHA-256
5. Render an HTML verification report
6. Exit 1 if any citation is unverified and --block-unverified is set

Silence is never success. Every citation produces an artifact.

## Install

    git clone https://github.com/mauludsadiq/CiteRight.git
    cd CiteRight
    cargo build --release

Requires Rust 1.76+.

## Quick Start

Offline mode (deterministic, CI-safe, no token required):

    cargo run -- verify-ai brief.md fixtures/courtlistener_fixture.json out/

Live mode (hits CourtListener API):

    cargo run -- verify-ai brief.md fixtures/courtlistener_fixture.json out/ --live --token $COURTLISTENER_TOKEN

Pre-filing gate (exits 1 if any citation unverified):

    cargo run -- verify-ai brief.md fixtures/courtlistener_fixture.json out/ --block-unverified --attorney-name "Jane Smith" --bar-number "CA-123456" --jurisdiction "California"

HTML report:

    cargo run -- report brief.md --offline-fixtures fixtures/courtlistener_fixture.json --out report.html

## verify-ai Flags

    --live                    Hit live CourtListener API instead of fixture
    --token <TOKEN>           CourtListener API token
    --endpoint <URL>          Override CourtListener endpoint
    --attorney-name <NAME>    Attorney name for audit receipt
    --bar-number <NUMBER>     Bar number for audit receipt
    --jurisdiction <NAME>     Jurisdiction for audit receipt
    --block-unverified        Exit 1 if any citation is unverified

## Verification Status

VERIFIED: citation resolved to a canonical CourtListener cluster
UNVERIFIED NOT_FOUND: no matching case found
UNVERIFIED AMBIGUOUS_MATCH: multiple possible cases, cannot resolve
UNVERIFIED API_ERROR: CourtListener returned an error
UNVERIFIED DIGEST_MISMATCH: response hash does not match expected

## Audit Receipt

Every verify-ai run produces an audit receipt at out/ai_audit_receipt.json containing:

- audit_id: SHA-256 over the full pipeline state
- input_digest: SHA-256 of the input document
- attorney attestation: name, bar number, jurisdiction, attestation text
- verified_count and unverified_count
- digests of claims, needs, candidates, resolutions, and selections
- receipt_digest: SHA-256 of the entire receipt

The attestation_text field is a plain-English statement suitable for attachment to a filing as an exhibit.

## HTML Report

    cargo run -- report brief.md --offline-fixtures fixtures/courtlistener_fixture.json --out report.html

Opens as a clean attorney-readable table showing status, case name, citation, date filed, canonical ID, and source link for every citation in the document.

## Testing

Run all tests:

    cargo test

Run live integration test (requires CourtListener token):

    COURTLISTENER_TOKEN=your_token cargo test --features live-tests

## Project Layout

    src/main.rs           CLI and command dispatch
    src/extract.rs        Citation extraction from documents
    src/claims.rs         Legal claim parsing
    src/planner.rs        Citation need planning
    src/candidates.rs     Candidate generation
    src/resolver.rs       Candidate resolution against fixtures
    src/selector.rs       Best candidate selection
    src/artifact.rs       VERIFIED/UNVERIFIED artifact production
    src/bindings.rs       Selection to artifact binding
    src/snapshot.rs       Live CourtListener API + snapshot normalization
    src/audit.rs          Attorney audit receipt generation
    src/report.rs         HTML verification report rendering
    src/courtlistener.rs  CourtListener API adapter
    src/document.rs       Document reading (md/txt/docx/pdf)
    src/models.rs         Shared data types
    src/hash.rs           SHA-256 helpers
    fixtures/             Offline fixture data for CI
    tests/                Integration tests

## Design Boundaries

CiteRight is not RAG. It does not evaluate whether a citation supports a legal proposition. It verifies that the cited authority exists in a canonical source.

A VERIFIED result means the case exists. It does not mean the quote is accurate, the holding applies, or the citation is used correctly. Those are attorney responsibilities.

CiteRight is the machine-verifiable layer beneath attorney judgment, not a replacement for it.

## Legal Notice

CiteRight is a verification tool, not legal advice.
