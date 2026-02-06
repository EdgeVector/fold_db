# A Peer‑to‑Peer, Mediated Model for Personal Data

## 1. Motivation

The modern internet is built on a structural asymmetry: individuals generate data, but institutions own, store, and monetize it. Personal data is fragmented across corporate silos, accessed through brittle APIs, duplicated endlessly, and governed by policies users neither see nor control.

This document outlines an alternative architecture: **personal data as a first‑class, user‑controlled system**, accessed peer‑to‑peer and mediated by AI rather than centralized platforms. The goal is not merely better privacy, but a simpler, more composable, and more *human‑aligned* data ecosystem.

At its core, the system treats personal data the way we treat people socially: you do not hand over your entire life to every party you interact with. Instead, you selectively reveal *insight*, *summaries*, or *claims*, often mediated by trust, context, and interpretation.

## 2. Core Principles

### 2.1 Personal Data Sovereignty

* Every individual controls their own data store.
* Data may be self‑hosted or hosted by a third‑party provider acting as a custodian, not an owner.
* Custodians are interchangeable; portability is a baseline requirement.

### 2.2 Peer‑to‑Peer Access

* Data access is fundamentally **connection‑based**, not globally addressable.
* To learn about someone, you must be connected to them (directly or via consented mediation).
* There is no universal database to query; there are only peers.

### 2.3 Mediated Disclosure

* Raw data is rarely shared.
* Insights are computed *in situ* via AI mediators.
* The requesting party receives answers, summaries, or proofs—not the underlying records.

### 2.4 Privacy‑Preserving Computation

* Computation moves to the data, not the other way around.
* Queries are evaluated under strict disclosure constraints.
* The system generalizes ideas from privacy computing, zero‑knowledge proofs, and secure enclaves, but prioritizes **practical expressiveness** over academic purity.

### 2.5 Global Schema, Local Stores

* All personal data conforms to a **shared global schema**.
* Each individual’s database is separate, private, and sovereign.
* Uniform shape enables universal tooling without centralization.

## 3. High‑Level Architecture

### 3.1 Personal Data Node (PDN)

Each individual operates (or delegates) a Personal Data Node:

* Encrypted storage of all personal data
* Schema‑validated data model
* Policy engine for access control
* AI mediator runtime
* Network interface for peer connections

The PDN is the *unit of identity* in the system.

### 3.2 AI Mediator Layer

AI mediators act as interpreters between raw data and external queries.

Responsibilities:

* Interpret incoming requests
* Apply disclosure policies
* Generate bounded responses (summaries, scores, explanations)
* Prevent data exfiltration through inference attacks

Crucially, mediators are **not neutral**—they are explicitly aligned to the data owner’s interests.

### 3.3 Peer Connection Graph

* Connections are explicit, consented, and revocable.
* Institutions (schools, banks, employers) are peers, not superusers.
* Trust is contextual, not absolute.

This mirrors social graphs more than client‑server models.

## 4. Data Access Model

### 4.1 Request Types

Requests fall into three broad categories:

1. **Direct Disclosure**

   * Explicitly shared fields (e.g., name, email)
2. **Derived Insight**

   * AI‑generated summaries or classifications
3. **Verified Claims**

   * Assertions with proofs (e.g., "income above X", "credit risk below Y")

### 4.2 Rumor‑Like Information Flow

The system intentionally mimics how information spreads among humans:

* You may know *that* something is true without knowing *why*
* You may receive a broad outline without intimate details
* Confidence levels and uncertainty are explicit

This is a feature, not a limitation.

### 4.3 Example: Mortgage Application

Instead of uploading bank statements, tax returns, and transaction histories:

* The lender submits a query: *"Assess creditworthiness under policy P"*
* The AI mediator evaluates local data
* The lender receives:

  * A risk score
  * Supporting explanations
  * Optional cryptographic attestations

Raw financial data never leaves the PDN.

## 5. Global Schema Design

### 5.1 Why a Global Schema Matters

Personal data is surprisingly uniform:

* Identity
* Relationships
* Finances
* Health
* Education
* Activity logs

Standardizing shape enables:

* Universal AI models
* Reusable queries
* Shared tooling
* Lower cognitive overhead

### 5.2 Schema Properties

* Extensible but opinionated
* Versioned and backward‑compatible
* Semantically rich, not just syntactic

### 5.3 Separation of Shape and Content

* **Shape** is global and shared
* **Content** is local and private

This is the key inversion compared to today’s platforms.

## 6. Comparison to Today’s Model

### 6.1 Current State

* Data lives in corporate silos
* Access via proprietary APIs
* Users grant blanket permissions
* Data is copied, cached, and resold
* Revocation is largely illusory

### 6.2 Proposed Model

* Data lives with the individual
* Access via peer connections
* Fine‑grained, query‑level disclosure
* No bulk data transfer
* Revocation is immediate and real

### 6.3 Complexity Inversion

Today:

* Simple for institutions
* Complex and opaque for users

Proposed:

* Slightly more complex for institutions
* Radically simpler and clearer for users

## 7. Security and Trust Model

### 7.1 Threat Assumptions

* Institutions are curious but rational
* Attackers may attempt inference, correlation, or replay attacks
* AI models are fallible and must be constrained

### 7.2 Defense Strategies

* Query rate limiting
* Differential disclosure
* Output entropy controls
* Auditable mediator decisions
* Cryptographic attestations where needed

Trust is **earned per interaction**, not granted once.

## 8. Open Questions and Extensions

* Standardization governance for the global schema
* Marketplaces for third‑party mediators
* Liability models for incorrect AI disclosures
* UX for explaining mediated answers to humans
* Interoperability with legacy systems

## 9. Conclusion

This system reframes personal data as something closer to *personhood* than property. It replaces bulk transfer with conversation, APIs with interpretation, and centralized trust with peer relationships.

The ambition is not to eliminate institutions or analytics, but to **realign power**: insight without exposure, access without ownership, and intelligence without surrender.

If successful, the internet becomes less like a set of data warehouses—and more like a network of people again.
