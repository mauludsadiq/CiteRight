# CiteRight

CiteRight is a Rust command-line tool that verifies legal citations in AI-drafted documents before filing. It catches two classes of AI hallucination that can expose attorneys to sanctions and malpractice liability.

The first class is citation existence failure: the case does not exist at all. The second class is citation mischaracterization: the case exists but does not support the proposition for which it is cited. The Sullivan and Cromwell AI hallucination incident involved the second class. CiteRight is built to catch both.

## How It Works

CiteRight operates in three layers.

Layer 1 is the Citation Grounding Layer. It extracts every citation from the input document, resolves each one against CourtListener, and produces an explicit VERIFIED or UNVERIFIED artifact. Silence is never success. The layer is fully deterministic, replayable, and produces a SHA-256 attorney-signed audit receipt bound to the document.

Layer 2 is the Legal Reasoning Layer. When --analyze is passed, CiteRight fetches the actual majority opinion from CourtListener, extracts real holdings using pattern matching, scores the applicability of each case to the argument deterministically, and uses GPT-4o to assess whether the brief claim matches what the case actually held.

Layer 3 is the Argument Validation Layer. It builds a Legal Argument Graph over all verified cases with typed edges (SUPPORTS, CONTRADICTS, DISTINGUISHES, CITES), validates each claim against the graph, detects conflicting authority, and produces a final VALID / PARTIALLY_VALID / INVALID / INDETERMINATE verdict sealed with a SHA-256 validation digest.

## Install

    git clone https://github.com/mauludsadiq/CiteRight.git
    cd CiteRight
    cargo build --release

Requires Rust 1.76+.

## Environment Variables

    COURTLISTENER_TOKEN    CourtListener API token (get one at courtlistener.com under Profile -> API)
    OPENAI_API_KEY         OpenAI API key (required for --analyze)
    RUST_LOG               Logging level, e.g. citeright=info or citeright=debug

Copy .env.example to .env and fill in your tokens.

## Quick Start

Offline mode (deterministic, CI-safe, no token required):

    cargo run -- verify-ai brief.md fixtures/courtlistener_fixture.json out/

Live mode with filing gate:

    cargo run -- verify-ai brief.md fixtures/courtlistener_fixture.json out/       --live       --block-unverified       --attorney-name "Jane Smith"       --bar-number "CA-123456"       --jurisdiction "California"

Full analysis pipeline:

    COURTLISTENER_TOKEN=your_token OPENAI_API_KEY=your_key     cargo run -- verify-ai brief.md fixtures/courtlistener_fixture.json out/       --live       --analyze       --block-unverified       --attorney-name "Jane Smith"       --bar-number "CA-123456"       --jurisdiction "California"

HTML report:

    cargo run -- report brief.md --offline-fixtures fixtures/courtlistener_fixture.json --out report.html

## verify-ai Flags

    --live                    Hit live CourtListener API instead of fixture
    --token <TOKEN>           CourtListener API token (or set COURTLISTENER_TOKEN)
    --endpoint <URL>          Override CourtListener endpoint
    --attorney-name <NAME>    Attorney name for audit receipt
    --bar-number <NUMBER>     Bar number for audit receipt
    --jurisdiction <NAME>     Jurisdiction for audit receipt
    --block-unverified        Exit 1 if any citation is unverified
    --analyze                 Run Layer 2 and 3: holding analysis, applicability scoring, argument graph, validation report

## Docker

    docker build -t citeright .
    docker run --rm       -e COURTLISTENER_TOKEN=your_token       -e OPENAI_API_KEY=your_key       -v $(pwd)/fixtures:/app/fixtures:ro       -v $(pwd)/input:/app/input:ro       -v $(pwd)/output:/app/output       citeright verify-ai         /app/input/brief.md         /app/fixtures/courtlistener_fixture.json         /app/output         --live --block-unverified

Or with docker-compose:

    cp .env.example .env
    mkdir input && cp your_brief.md input/brief.md
    docker-compose up

## Verification Status

    VERIFIED                  Citation resolved to a canonical CourtListener cluster
    UNVERIFIED NOT_FOUND      No matching case found
    UNVERIFIED AMBIGUOUS      Multiple possible cases, cannot resolve
    UNVERIFIED API_ERROR      CourtListener returned an error
    UNVERIFIED DIGEST_MISMATCH Response hash does not match expected

## Full Pipeline Output (--analyze)

Running with --analyze produces four structured outputs:

Holding Analysis: what each verified case actually held, extracted from live majority opinion text and assessed by GPT-4o against the brief claim.

Applicability Scores: deterministic scoring of how relevant each case is to the argument, using keyword overlap, legal concept matching, rule alignment, and recency. No LLM required.

Argument Graph: a structured graph of case nodes with typed edges and claims mapped to cited cases. Sealed with a SHA-256 graph digest.

Validation Report: the final verdict. Each claim is rated SUPPORTED, PARTIALLY_SUPPORTED, UNSUPPORTED, or UNVERIFIABLE. The overall argument is rated VALID, PARTIALLY_VALID, INVALID, or INDETERMINATE. Conflicting authority is flagged explicitly. Sealed with a SHA-256 validation digest.

Example from a real run:

    Obergefell v. Hodges: Supported (High confidence)
    Conti v. Perdue Bioenergy: Unsupported (High confidence) -- commodity forward agreement holding
    does not support constitutional guarantees claim.

    Overall: PARTIALLY_VALID
    Summary: 1/2 claims supported. 1 unsupported claim flagged.

## Audit Receipt

Every verify-ai run writes out/ai_audit_receipt.json containing the audit_id (SHA-256 over the full pipeline state), input_digest (SHA-256 of the input document), attorney attestation with name, bar number, jurisdiction and attestation text, verified and unverified counts, digests of every pipeline stage, and a receipt_digest over the entire receipt. The attestation_text is suitable for attachment to a filing as an exhibit.

## Testing

    cargo test

Run live integration test (requires CourtListener token):

    COURTLISTENER_TOKEN=your_token cargo test --features live-tests

## CI

GitHub Actions runs on every push and pull request: cargo test, cargo clippy -D warnings, and release binary builds for Linux x86_64 and macOS ARM64.

## Project Layout

    src/main.rs                     CLI and command dispatch
    src/extract.rs                  Citation extraction from documents
    src/claims.rs                   Legal claim parsing
    src/planner.rs                  Citation need planning
    src/candidates.rs               Candidate generation
    src/resolver.rs                 Candidate resolution against fixtures
    src/selector.rs                 Best candidate selection
    src/artifact.rs                 VERIFIED/UNVERIFIED artifact production
    src/bindings.rs                 Selection to artifact binding
    src/snapshot.rs                 Live CourtListener API + snapshot normalization
    src/audit.rs                    Attorney audit receipt generation
    src/report.rs                   HTML verification report rendering
    src/courtlistener.rs            CourtListener API adapter
    src/document.rs                 Document reading (md/txt/docx/pdf)
    src/models.rs                   Shared data types
    src/hash.rs                     SHA-256 helpers
    src/reasoning/holdings_extractor.rs   Pattern-based holding extraction from opinion HTML
    src/reasoning/analyzer.rs             Live opinion fetch + GPT-4o claim assessment
    src/reasoning/applicability.rs        Deterministic applicability scoring
    src/reasoning/argument_graph.rs       Legal argument graph with typed edges
    src/reasoning/validation.rs           Argument validation and overall verdict
    fixtures/                       Offline fixture data for CI
    tests/                          Integration tests

## Web Interface

CiteRight includes a web interface for attorneys who prefer not to use the CLI.

Run locally:

    cargo run --features server --bin citeright-server

Then open http://localhost:3000 in your browser. Upload a legal document, optionally enter attorney attestation details, and click Verify Citations. The interface returns verified/unverified status for every citation and a signed audit receipt.

Configure with environment variables:

    CITERIGHT_PORT       Port to listen on (default: 3000)
    CITERIGHT_FIXTURE    Path to offline fixture file (default: fixtures/courtlistener_fixture.json)
    COURTLISTENER_TOKEN  CourtListener API token for live mode
    OPENAI_API_KEY       OpenAI API key for --analyze

Run with Docker:

    docker build -t citeright .
    docker run --rm -p 3000:3000 \
      -e COURTLISTENER_TOKEN=your_token \
      -e OPENAI_API_KEY=your_key \
      citeright-server

## Deployment

CiteRight server deploys as a single Docker container. It has been tested on Railway.

For law firms requiring on-premise deployment, run the Docker container on any internal server. Data never leaves the firm network. Only the audit receipt JSON is produced as output — no document content is stored or logged.

For managed deployment, set COURTLISTENER_TOKEN and OPENAI_API_KEY as environment variables in your hosting platform.

## Legal Notice

CiteRight is a verification and analysis tool, not legal advice. A VERIFIED result means the case exists. A Supported assessment means an LLM found the claim consistent with the extracted holding. Neither constitutes legal advice or guarantees the citation is used correctly in context. Attorney judgment remains required.
