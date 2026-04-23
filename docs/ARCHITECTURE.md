# Cite Right Architecture

Cite Right treats a legal brief as a graph of claims. A citation claim has no downstream existence until it resolves to a canonical source artifact.

## Components

1. `document`: extracts text from `.txt`, `.md`, `.docx`, and `.pdf`.
2. `extract`: finds legal citation candidates and emits claim IDs.
3. `courtlistener`: resolves claims through CourtListener's citation lookup API or deterministic offline fixtures.
4. `verify`: classifies each claim as VERIFIED, HEALED, AMBIGUOUS, or BLOCKED.
5. `emit`: writes verified markdown and machine-readable JSON artifacts.
6. `hash`: creates SHA-256 commitments for records and receipts.

## Invariant

No citation enters verified output unless it resolves to a canonical artifact.

## Failure Modes

- `404`: blocked, no canonical source found.
- `400`: blocked, invalid reporter or malformed citation.
- `300`: ambiguous, human legal review required.
- `429`: blocked, lookup was throttled or citation batch exceeded source limits.

## Artifact Healing

Healing is allowed only when the lookup source returns status `200`, exactly one cluster, and exactly one normalized citation that differs from the raw citation. Otherwise the claim is blocked or marked ambiguous.
