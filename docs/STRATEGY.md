# Strategy: Transforming FoldDB to the P2P Personal Data Vision

This document outlines the strategic roadmap for evolving FoldDB from its current **centralized cloud architecture** into the **peer-to-peer, mediated model for personal data** described in [GOAL.md](./GOAL.md).

---

## Executive Summary

The goal state describes a decentralized network where individuals operate Personal Data Nodes (PDNs), data access is peer-to-peer, and AI mediators compute insights _in situ_ rather than transferring raw data. The current FoldDB implementation is a centralized multi-tenant cloud platform. This strategy bridges that gap through **four major phases**, progressively decentralizing data ownership while preserving what works today.

---

## Current State Analysis

### What We Have

| Capability             | Current Implementation                                       |
| :--------------------- | :----------------------------------------------------------- |
| **Data Storage**       | Centralized DynamoDB with tenant partitioning by `user_hash` |
| **Identity**           | Pseudonymous passkey-based auth (WebAuthn)                   |
| **Multi-tenancy**      | Strict "Zero Fallback Keys" isolation per user               |
| **Schema**             | Global Schema Registry at `schema.folddb.com`                |
| **AI Queries**         | `LlmQueryService` for semantic search and summarization      |
| **Transport**          | AWS Lambda + API Gateway, standalone Actix-web server        |
| **Third-Party Access** | Planned: Passkey-native auth proxy with scoped JWTs          |

### What the Goal Requires

| Capability             | Goal State Description                               |
| :--------------------- | :--------------------------------------------------- |
| **Data Storage**       | Local Personal Data Nodes (self-hosted or custodial) |
| **Identity**           | PDN _is_ identity; connection-based trust            |
| **Multi-tenancy**      | N/A — each user operates their own node              |
| **Schema**             | Global schema, local stores (already aligned)        |
| **AI Queries**         | AI Mediators aligned to data owner's interests       |
| **Transport**          | P2P network layer; no central server                 |
| **Third-Party Access** | Peer connections with query-level disclosure         |

---

## Gap Analysis

### Critical Gaps

1. **No P2P Network Layer** — The system lacks peer discovery, connection management, and direct node-to-node communication.

2. **No Policy Engine** — The goal requires fine-grained disclosure policies; we have only coarse API-level permissions.

3. **Limited Mediator Capabilities** — The current LLM service answers questions but doesn't _filter disclosure_ or detect inference attacks.

4. **No Derived Insight Pipeline** — We lack mechanisms for generating verified claims, risk scores, or cryptographic attestations.

5. **No Local-First Runtime** — The current standalone server assumes cloud backends; true PDNs need a fully local data layer.

### Existing Strengths (Keep)

- **Global Schema Registry** — Already implements "Global Shape, Local Content"
- **Schema Isolation** — Multi-tenant enforcement can evolve into PDN isolation
- **LLM Query Service** — Foundation for AI mediator layer
- **Passkey Identity** — Aligns with cryptographic identity model

---

## Strategic Phases

### Phase 1: Local-First Personal Data Node

**Goal**: Enable true self-hosting with no cloud dependency

```
┌─────────────────────────────────────────────────────┐
│                   Local PDN                         │
│  ┌─────────┐  ┌─────────────┐  ┌────────────────┐  │
│  │ Storage │  │   Schema    │  │ AI Mediator    │  │
│  │ (Sled)  │  │  Registry   │  │   (Local LLM)  │  │
│  └─────────┘  └─────────────┘  └────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**Deliverables**:

- [ ] Local-only Sled storage as default (no DynamoDB)
- [ ] Embedded schema registry (sync from global on demand)
- [ ] Local LLM integration (Ollama, llama.cpp) for offline mediator
- [ ] Desktop/CLI distribution for personal hosting

**Preserves**: Ingestion pipeline, query engine, schema system

---

### Phase 2: Policy Engine and Disclosure Controls

**Goal**: Users define what can be revealed and to whom

```
┌─────────────────────────────────────────┐
│            Disclosure Policy            │
├─────────────────────────────────────────┤
│ WHO: @bank_peer                         │
│ WHAT: income_summary, credit_indicators │
│ HOW: aggregated (no raw records)        │
│ WHEN: valid until 2026-03-01            │
└─────────────────────────────────────────┘
```

**Deliverables**:

- [ ] Policy definition language (YAML/JSON DSL)
- [ ] Schema-aware disclosure rules (field-level controls)
- [ ] Query-time policy enforcement in AI mediator
- [ ] Audit log of all disclosures
- [ ] Revocation mechanisms with immediate effect

**New Concept**: Policies replace API permissions

---

### Phase 3: Connection Graph and P2P Transport

**Goal**: Replace client-server with peer connections

```
     ┌─────┐          ┌─────┐
     │PDN A│◄────────►│PDN B│
     └──┬──┘          └──┬──┘
        │                │
        │    ┌─────┐     │
        └───►│PDN C│◄────┘
             └─────┘
```

**Deliverables**:

- [ ] libp2p or similar transport layer
- [ ] Peer discovery (bootstrap nodes, DHT, or invite codes)
- [ ] Connection handshake with mutual authentication
- [ ] Encrypted channels (Noise protocol or similar)
- [ ] Revocable connections with cascade policies

**Major Shift**: Institutions become _peers_, not privileged endpoints

---

### Phase 4: AI Mediator as Gatekeeper

**Goal**: AI computes insights without leaking raw data

```
               Query: "Assess creditworthiness"
                          │
                          ▼
              ┌───────────────────────┐
              │     AI Mediator       │
              │  • Interpret request  │
              │  • Check policy       │
              │  • Compute locally    │
              │  • Enforce disclosure │
              └───────────────────────┘
                          │
                          ▼
               Response: Score + Explanation
               (No raw financial data sent)
```

**Deliverables**:

- [ ] Query classification (direct / derived / verified claim)
- [ ] Inference attack detection (rate limits, entropy monitoring)
- [ ] Output guards (minimum aggregation, differential privacy hints)
- [ ] Cryptographic attestations (signed claims, ZK proofs for specific assertions)
- [ ] Mediator alignment: explicit owner-interest prioritization

**Key Insight**: Raw data rarely leaves the node

---

## Migration Strategy

### Hybrid Transition Period

Users can operate in three modes during transition:

| Mode              | Description                          | Use Case                              |
| :---------------- | :----------------------------------- | :------------------------------------ |
| **Cloud-Managed** | Current DynamoDB-backed hosting      | Users who prefer custodial simplicity |
| **Self-Hosted**   | Local PDN with optional cloud sync   | Privacy-conscious users               |
| **Federated**     | Custodian hosts PDN on user's behalf | Intermediate trust model              |

All modes share the same schema registry and protocol.

### Data Portability Guarantee

```bash
# Export entire PDN to portable format
fold_db export --format pdn-bundle --output my_data.zip

# Import to new node (self-hosted or different custodian)
fold_db import --from my_data.zip --verify-signatures
```

---

## Verification Milestones

### Milestone 1: Local-First Demo

- [ ] `fold_db` runs fully offline
- [ ] Ingest files, query via AI, no network calls
- [ ] Embed schema locally

### Milestone 2: Policy Enforcement Demo

- [ ] Define policy restricting field access
- [ ] Query returns filtered/aggregated data per policy
- [ ] Audit log captures disclosure event

### Milestone 3: Two-Node P2P Demo

- [ ] Two PDNs discover and connect
- [ ] PDN A queries PDN B with permission
- [ ] Response computed by B's mediator, sent to A

### Milestone 4: Mortgage Application Scenario

- [ ] User PDN receives "assess creditworthiness" query
- [ ] AI mediator computes risk score
- [ ] Only score + explanation returned
- [ ] Raw financial data never transmitted

---

## Open Questions (Require Design Decisions)

1. **Schema Governance**: Who controls the global schema? DAO? Foundation? Community RFC?

2. **Custodian Liability**: If a hosting provider acts as custodian, what legal/technical guarantees?

3. **Mediator Marketplace**: Allow third-party AI mediators? How to verify alignment?

4. **Backward Compatibility**: Can existing Exemem cloud users transition seamlessly?

5. **Performance**: P2P queries will have higher latency — how to cache or optimize?

---

## Recommended Next Steps

### Immediate (This Quarter)

1. **Spike: Local-Only Mode** — Get `fold_db` running with Sled-only storage, no AWS dependencies
2. **Design: Policy DSL** — Draft the disclosure policy language spec
3. **Research: P2P Libraries** — Evaluate libp2p, Holepunch, or custom transport

### Near-Term (Next Quarter)

4. **Prototype: Policy Engine** — Implement field-level disclosure in mediator
5. **Prototype: Two-Node Demo** — Basic P2P connection and query forwarding

### Medium-Term (6 Months)

6. **Production: Local-First Distribution** — Desktop app or Docker container
7. **Production: Policy UI** — User-friendly disclosure rule editor

---

## Conclusion

FoldDB has a solid foundation for the goal state:

- **Global schemas** already implement the "shape is public, content is private" principle
- **Multi-tenant isolation** can evolve into per-node sovereignty
- **LLM Query Service** is the embryonic AI mediator

The journey requires major new capabilities — P2P networking, policy enforcement, and aligned mediation — but the path is incremental. Each phase delivers standalone value while moving toward the ultimate vision of **personal data as personhood**.

---

_"The ambition is not to eliminate institutions or analytics, but to realign power."_
