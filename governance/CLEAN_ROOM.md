# Clean-room reference rule

The projects in `governance/provenance.json` are references, not source pools. A reference may inform requirements, observable behavior, architecture tradeoffs, and independently written tests. Reuse or adaptation of implementation material requires an explicit provenance entry and license review before merge.

Plane is licensed under AGPL-3.0-only at the recorded reference revision. Plane source code, pseudocode, comments, tests, schemas, styles, assets, and other implementation expression must not be copied into Jet Black and must not be translated line-by-line, mechanically, or semantically into another language or framework.

When implementing behavior observed in Plane:

1. Record the behavior as an implementation-neutral requirement without retaining Plane source excerpts.
2. Have the implementer work from that requirement and Jet Black's own contracts and conventions.
3. Use independently authored names, structure, control flow, tests, and documentation.
4. Record any permitted third-party adaptation separately, including the exact source revision, files, license, notices, and reviewer decision.
5. Stop and request review when provenance or license compatibility is uncertain.

This rule does not select a license for Jet Black and does not declare the project ready for distribution.
