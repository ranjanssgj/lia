# Supply-Chain, SBOM, Licensing and Release Contract v1

**Status:** Draft — SUPPLY-01 evidence pending

## 1. Component inventory

Every release inventory covers direct and transitive dependencies plus non-code deliverables:
- Rust crates, Node packages, native system libraries and generated protobuf clients;
- Godot engine/export templates, plugins, VRM/runtime components and native extensions;
- fonts, audio, voice assets, character models, animations, textures and other bundled media;
- model weights, tokenizer files, embedding assets and speech assets if any are later bundled;
- container base images, build tools, CI actions, package managers and build-time generators.

Each component records name, version, supplier, source, package URL where available, license expression, copyright/notice, hash, dependency relationship, build role and any modification/patch. Vendored or generated components retain origin and toolchain provenance.

## 2. SBOM format and publication

The minimum release SBOM format is SPDX 2.3 JSON. A CycloneDX SBOM may be added for tool compatibility but does not replace the required license/notice review. The SBOM is generated from the actual release artifact and lockfiles, not manually assembled from the top-level manifest.

The release stores:
- source revision and build provenance reference;
- per-target artifact hashes and immutable image digests;
- SBOM file/hash and tool version;
- license notices and third-party attribution bundle;
- vulnerability scan report and disposition;
- signing identity/key ID and artifact signature verification result;
- asset/model/audio/font rights records and source/license files.

No release claim is made until the SBOM is generated from the actual binaries/images/assets.

## 3. License and data-rights review

Every dependency and asset is reviewed for license, redistribution, modification, attribution, commercial use, source disclosure, model-weight restrictions, voice likeness/consent, character/rig/animation rights and user-data restrictions. A transitive component cannot be ignored because it is bundled indirectly by Godot, a crate or an SDK.

Unknown, conflicting, non-redistributable or incompatible terms block distribution until an explicit legal/owner decision is recorded. Third-party licenses are not inferred from the license of the containing project. User-provided assets also need provenance and an authorization record appropriate to their intended distribution.

## 4. Vulnerability and update management

Before release:
- scan source dependencies, native libraries, container images and packaged assets where scanners support them;
- triage known vulnerabilities for exploitability, reachability, impact and available remediation;
- block unresolved critical/high issues unless a documented, time-bounded exception is approved by the designated owner;
- record patch, upgrade, mitigation or exception rationale;
- rebuild and rescan the release after dependency/security updates.

A vulnerability exception states the affected component/version, CVE/advisory, exploitability analysis, compensating controls, owner, approval, expiry/review date and remediation plan. Exceptions do not silently become permanent.

## 5. Build provenance, signing and rollback

Builds are reproducible where the toolchain permits and produce immutable artifacts with source revision, builder identity, dependency lock hashes and build attestation. Signing keys are held by the selected release-signing/keystore boundary and are not exposed to ordinary CI jobs. The concrete signer/vendor remains open.

Updates verify signature, target OS/architecture, version compatibility and artifact digest before installation. Staged rollout monitors startup/readiness and contract compatibility. Rollback restores compatible binaries/configuration but preserves monotonic security state, revocation/deletion tombstones, consumed nonces/leases, workflow history and external side effects.

## 6. SUPPLY-01 acceptance

Retain artifact-derived SPDX SBOM, license/notice bundle, asset/model/voice rights inventory, dependency tree and lock hashes, vulnerability report/disposition, build provenance, signature-verification output, update/rollback drill and reviewer sign-off for each distributable profile.
