# Security Model

This document orients a security reviewer — human or automated/LLM agent — to
**where the code that needs auditing actually lives** in this repository, and to
the trust-boundary references that apply to it.

It is intentionally brief. It is a **map to scanning targets**, not a full threat
model. The detailed, per-platform trust models live in the SDK repositories (see
[§3](#3-which-trust-model-applies)).

---

## 1. The `main` branch has no TEE code

`main` is the **landing page** for the Teaclave ecosystem. Its `crates/`, `sgx/`,
and `trustzone/` directories contain only `README.md` showcase pointers — there
is **no enclave, no Trusted Application, and nothing compiled into a TEE** on
this branch. There is therefore no trust boundary or Trusted Computing Base to
audit on `main` itself. A security scan of the Teaclave codebase should target
the branches below, not `main`.

---

## 2. Scanning targets (branches that contain code)

| Branch | Status | Where the code is | What it is |
|---|---|---|---|
| **`experimental-web3`** | **Active / latest** | `vm/wallet-gateway/` | A TEE-backed crypto-wallet custody gateway — the current focus. **Start here.** |
| `cleanroom` | Prototype (2022) | repo root (`attestation/`, `services/`, `executor/`, `dcap/`, `edl/`, `function/`, `rpc/`, `sdk/` …) | An earlier SGX FaaS-style platform prototype. |
| `legacy` | Deprecated | `services/`, `docs/` | The original Teaclave FaaS framework. Already carries its own security docs: `docs/threat-model.md`, `docs/mutual-attestation.md`, `docs/service-internals.md`, `docs/access-control.md`. |

### 2.1 `experimental-web3` → `vm/wallet-gateway/` (primary target)

A multi-crate wallet gateway that custodies keys and signs blockchain
transactions inside a TEE. The **trust boundary is visible in the workspace
layout** — the task runner is split into a trusted and an untrusted half:

- **Trusted (inside the TEE):** `task-runner-tee` and the key/credential material
  it handles (`credential-manager`, the signing/wallet logic). This is the TCB —
  private keys and plaintext must never leave it.
- **Untrusted (host / normal world):** `api-server` and `webapi` (the public
  client-facing edge), `task-runner-normal`, `db-service` / `db-manager`
  (persistence in untrusted storage), `net` (outbound calls to **external,
  attacker-influenced** services such as blockchain RPC — `net/.../btc_rpc.rs` —
  and price oracles — `net/.../asset_price.rs`), and `authority-server`'s host
  surface.

**Attacker-controlled inputs a scanner should focus on:**
- Every client request entering `api-server` / `webapi`.
- Every response from `net` (blockchain RPC, price feeds) — external data is
  untrusted; never base a signing or balance decision on it without validation.
- Everything read back from `db-service` / `db-manager` — persisted data lives in
  untrusted storage and must be encrypted/authenticated; treat reads as
  adversarial.
- The boundary crossing between `task-runner-normal` and `task-runner-tee`.

Key questions: do wallet private keys and unencrypted secrets ever cross out of
`task-runner-tee`? Is data persisted by `db-service` sealed/authenticated before
it leaves the TEE? Are external feed responses validated before they influence a
signature or a balance?

### 2.2 `cleanroom` and `legacy`

Both are SGX FaaS-style platforms (frontend / management / storage / scheduler /
execution / access-control services, each split into an untrusted `app/` wrapper
and a trusted `enclave/`, joined by mutually-attested TLS). `legacy` already
documents its trust model under `docs/`; `cleanroom` is an earlier prototype of
the same shape. Audit these against the SGX trust model below; do not duplicate
`legacy`'s existing threat-model docs.

---

## 3. Which trust model applies

The applications on these branches are built on the Teaclave TEE SDKs, so the
**authoritative trust-boundary methodology is the SDK security models** — use
them as the reference when auditing:

- **Intel SGX targets** (the FaaS platforms, and SGX builds of the gateway):
  [Teaclave SGX SDK → `docs/security-model.md`](https://github.com/apache/teaclave-sgx-sdk/blob/main/docs/security-model.md)
  — untrusted host vs. enclave, the ECALL/OCALL edge, OCALL results are
  untrusted.
- **Arm TrustZone targets:**
  [Teaclave TrustZone SDK → `docs/security-model.md`](https://github.com/apache/teaclave-trustzone-sdk/blob/main/docs/security-model.md)
  — Normal World vs. Secure World, treat all parameters crossing into the TA as
  attacker-controlled.
- **Dependencies linked into any of the above:**
  [Teaclave Dependency Crates → `docs/security-model.md`](https://github.com/apache/teaclave-crates/blob/main/docs/security-model.md)
  — everything linked into the trusted side is part of the TCB.

The common rule across all of them: **the TEE side is trusted; everything else —
the host OS, clients, the network, external feeds, and persisted storage — is
attacker-controlled and must be validated at the boundary.**

---

## 4. Reporting vulnerabilities

Security issues should be reported privately first, per
[`SECURITY.md`](../SECURITY.md), before any public disclosure.
