# Reference reuse rule

The projects in `governance/provenance.json` are reusable source material.
Inspect their code whenever it can reduce uncertainty or avoid rebuilding an
already solved capability.

When a reference implementation matches Jet Black's requirement:

1. Prefer copying or adapting it over rewriting it only to make the code
   different.
2. Adjust language, runtime, domain, transport, security, and UI boundaries only
   where Jet Black requires it.
3. Record the source project, exact revision, and relevant files for substantial
   adaptations.
4. Preserve or translate useful behavioral tests and failure cases.
5. Keep Jet Black's accepted ADRs authoritative when a reference makes a
   different product or trust decision.

This register is for traceability. It does not require independent or
clean-room reimplementation.
