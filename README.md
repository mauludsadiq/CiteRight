# Cite Right

**Cite Right** is a Rust legal citation proof gate for AI-drafted legal text.

It converts legal citations from probabilistic strings into deterministic verification artifacts. A citation that cannot be resolved to a canonical legal source is blocked or redacted before downstream review.

## Core Rule

```text
Verify or the object does not exist.
```

## What It Does

Cite Right takes a legal document, extracts citations, resolves them through a canonical source adapter, classifies each citation, and writes a receipt chain.

Supported input formats:

- `.txt`
- `.md`
- `.docx`
- `.pdf`

Primary canonical source adapter:

- CourtListener `/api/rest/v4/citation-lookup/`

CourtListener documents that this endpoint can look up individual citations or parse citations from blocks of text, and that it is useful as a guardrail to prevent hallucinated citations. It returns statuses including `200`, `300`, `400`, `404`, and `429`.

## Output Artifacts

```text
out/legal_gate_run/
  verified_brief.md
  failed_claims.json
  citations.json
  receipt_chain.json
  run_receipt.json
```

## Install / Build

```bash
cargo build
```

## Run With Offline Fixture

This mode is deterministic and does not require a CourtListener token.

```bash
cargo run -- verify fixtures/sample_brief.md --out out/sample --offline-fixtures fixtures/courtlistener_fixture.json
```

Expected behavior:

- `576 U.S. 644` is verified.
- `540 B.R. 201` is blocked.
- `576 US 644` is healed to `576 U.S. 644`.

## Run Against CourtListener

CourtListener's API uses token authentication for REST calls.

```bash
export COURTLISTENER_TOKEN="your-token-here"
cargo run -- verify brief.docx --out out/legal_gate_run
```

You may also pass the token explicitly:

```bash
cargo run -- verify brief.pdf --out out/legal_gate_run --courtlistener-token "your-token-here"
```

## CLI

```text
citeright verify <INPUT> --out <DIR> [--courtlistener-token <TOKEN>] [--offline-fixtures <JSON>]
citeright extract <INPUT>
```

## Verification States

| State | Meaning | Action |
|---|---|---|
| `VERIFIED` | Citation resolved to exactly one canonical artifact | Keep |
| `HEALED` | Citation had a unique normalized correction | Replace with normalized form |
| `AMBIGUOUS` | Multiple possible canonical artifacts | Block / human review |
| `BLOCKED` | No canonical source or invalid citation | Block / redact |

## Receipt Model

Each claim receives a deterministic receipt digest over:

- raw citation
- normalized citation candidates
- canonical source response
- verification status
- selected action
- reason

The run receipt commits to:

- input digest
- policy
- claim counts
- output artifact paths
- full receipt chain digest

Example failure:

```json
{
  "raw": "540 B.R. 201",
  "status": "BLOCKED",
  "reason": "no canonical source found",
  "action": "BLOCK_OR_REDACT"
}
```

## VS Code

Open the unzipped folder in VS Code. The workspace includes:

- `.vscode/settings.json`
- `.vscode/tasks.json`
- `.vscode/launch.json`

Available tasks:

- `cargo build`
- `cargo test`
- `verify sample`

## Design Boundaries

This is not RAG. It does not ask whether text sounds legal. It asks whether each citation resolves to a canonical artifact.

The app is adapter-based. CourtListener is the first source. Westlaw, Lexis, PACER, or local licensed corpora can be added by implementing the same lookup boundary.

## Project Layout

```text
src/
  main.rs           CLI and run orchestration
  document.rs       txt/md/docx/pdf extraction
  extract.rs        legal citation candidate extraction
  courtlistener.rs  canonical source adapter + fixtures
  verify.rs         proof gate classification
  emit.rs           verified markdown + JSON artifact writing
  hash.rs           SHA-256 helpers
  models.rs         serializable contracts
fixtures/
  sample_brief.md
  courtlistener_fixture.json
docs/
  ARCHITECTURE.md
.vscode/
  settings.json
  tasks.json
  launch.json
```

## Legal Use Warning

Cite Right is a verification tool, not legal advice. A `VERIFIED` result means the cited authority exists in the configured canonical source. It does not prove that the authority supports the legal proposition.
