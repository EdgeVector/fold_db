# Documentation Index

This directory now centralizes product knowledge, architecture references, and project planning material. Use the sections below
to quickly locate the document type you need.

## Directory Overview
- `overview/` – Core concept introductions such as architecture, system overview, and representative use cases.
- `guides/` – How-to material organized by topic:
  - `development/` – Engineering workflows, debugging references, and language-specific guides.
  - `operations/` – Deployment, migration, and operational readiness procedures.
  - `testing/` – Coverage instructions and deep dives into testing strategies.
  - `tooling/` – Notes for repository utilities and helper scripts.
- `reference/` – API definitions, schema specifications, and formal requirements.
- `security/` – Reviews, remediation plans, and access control guidance.
- `transforms/` – Documentation for transform formats, execution, and reviews.
- `ingestion/` – Large-scale ingestion design references.
- `ui/` – Frontend alignment notes, testing guides, and the static React documentation set.
- `network/` – Node behavior reviews and transport layer documentation.
- `examples/` – Sample payloads, data walk-throughs, and API usage examples.
- `design/` – Diagrams and design explorations for critical subsystems.
- `delivery/` – PBI definitions, task trackers, and historical delivery records.
- `proposals/` – Product proposals, including the consolidated social media ingestion plan.
- `assets/` – Shared styling and rendered HTML artifacts used by other documents.
- `project_logic.md` – Authoritative record of cross-cutting logic decisions.

## Recently Consolidated Documents
- Repository-wide plans such as the debugging plan, Rust package guidance, and coverage setup now live under `guides/`.
- UI review reports, static React architecture notes, and migration guidance have been grouped under `ui/`.
- Module-specific READMEs (scripts, samples, payment requirements, and network notes) have moved into `guides/`, `examples/`,
  or `network/` according to their scope.

Keeping everything under `docs/` eliminates stray Markdown files elsewhere in the repository and makes it easier to discover
related information.
