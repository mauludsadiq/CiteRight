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

## Holding Analysis (--analyze)

When you add --analyze, CiteRight goes beyond existence verification into Layer 2: it fetches the actual majority opinion from CourtListener, extracts the real holdings using pattern matching, and asks an LLM whether the brief's claim matches what the case actually held.

    cargo run -- verify-ai brief.md fixtures/ out/ \
      --live --token $COURTLISTENER_TOKEN \
      --analyze \
      --block-unverified

This catches two classes of AI hallucination:

The first is citation existence failure -- the case does not exist at all. The second is citation mischaracterization -- the case exists but does not support the proposition for which it is cited. The Sullivan and Cromwell incident involved the second class. --analyze is built to catch it.

Example output from a real run against Obergefell v. Hodges and Conti v. Perdue Bioenergy:

    Obergefell: Supported (High confidence)
    "The claim accurately reflects the holding... constitutional guarantees cannot be overridden by state laws."

    Conti: Unsupported (High confidence)
    "The claim about constitutional guarantees has nothing to do with Conti's actual holding on commodity forward agreements under 546(g)."

Set your OpenAI API key to enable assessment:

    export OPENAI_API_KEY=your_key_here

Holdings extraction runs without an API key. LLM assessment requires one.

## Argument Graph

When --analyze runs, CiteRight builds a Legal Argument Graph over the verified citations. The graph is a structured representation of the legal argument with cryptographic integrity.

Each verified case becomes a CaseNode containing the canonical ID, case name, and extracted holdings. Claims from the brief are mapped to the cases they cite. Edges between cases represent typed legal relationships: SUPPORTS, CONTRADICTS, DISTINGUISHES, or CITES. The entire graph is sealed with a SHA-256 digest.

Example graph output:

    {
      "nodes": {
        "cluster:2812209": {
          "canonical_id": "cluster:2812209",
          "case_name": "Obergefell v. Hodges",
          "holdings": ["We hold that same-sex couples may exercise the fundamental right to marry."],
          "rule_extracted": "We hold that same-sex couples may exercise the fundamental right to marry."
        }
      },
      "edges": [],
      "claims": [
        {
          "claim_id": "cluster:2812209",
          "cited_case_ids": ["cluster:2812209"],
          "assessment": "Supported: The claim accurately reflects the holding..."
        }
      ],
      "graph_digest": "sha256:..."
    }

The graph_digest commits the entire argument structure -- nodes, edges, and claim assessments -- to a single verifiable hash. This means the legal reasoning output is as auditable as the citation existence layer beneath it.

## Architecture

CiteRight is built in two layers.

Layer 1 is the Citation Grounding Layer. It is deterministic, replayable, and non-LLM-dependent. It extracts citations, resolves cases, binds to canonical artifacts, verifies existence, and produces SHA-256 audit receipts. This layer cannot be wrong about facts.

Layer 2 is the Legal Reasoning Layer. It fetches majority opinion text, extracts structured holdings, and uses an LLM to assess whether the brief's claim matches the holding. This layer can be wrong -- it is advisory, not authoritative. The grounding layer beneath it remains deterministic regardless of what the reasoning layer concludes.

## Legal Notice

CiteRight is a verification tool, not legal advice.
