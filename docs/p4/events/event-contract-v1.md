# LIA Domain Event Contract v1

**Status:** Draft — NATS-01 proof pending  
**Transport:** NATS JetStream  
**Canonical durability:** Owning-domain transaction + PostgreSQL outbox/inbox  
**Delivery semantics:** At least once

## 1. Event meaning

An event is a recorded fact or observation. It is not a command, grant, approval, or instruction. Event content cannot widen permissions, alter capability manifests, change identity, or directly execute an operation. Commands are submitted to the owning authority and validated separately.

A domain event is publishable only after its owning state transition and outbox record have committed. The broker is transport; it is not the canonical event store or workflow authority.

## 2. Envelope

Every event uses the versioned JSON envelope below. The event type selects a versioned payload schema; consumers reject or quarantine incompatible required semantics.

~~~json
{
  "event_id": "uuid",
  "event_type": "device.presence.changed",
  "schema_version": "1",
  "account_id": "uuid",
  "aggregate": {
    "type": "device",
    "id": "uuid",
    "sequence": 42
  },
  "producer": {
    "component_id": "device-registry",
    "component_version": "semver",
    "principal_type": "service"
  },
  "source_event_id": "optional provider/device event ID",
  "occurred_at": "RFC3339 source occurrence time",
  "recorded_at": "RFC3339 canonical commit time",
  "correlation_id": "uuid",
  "causation_id": "uuid-or-null",
  "trust": {
    "source_authenticated": true,
    "authentication_context_ref": "opaque-reference"
  },
  "classification": {
    "sensitivity": "S0",
    "scope": "account",
    "retention_class": "transport_transient"
  },
  "payload_digest_sha256": "hex-encoded SHA-256",
  "data": {}
}
~~~

Envelope field requirements:
- event_id is globally unique and immutable.
- event_type and schema_version identify a specific payload contract.
- account_id is the tenant boundary; it is not an authentication credential.
- aggregate.sequence is monotonic within that aggregate only. There is no global event order.
- occurred_at is source time; recorded_at is the canonical owner's commit/receipt time. Coordinator time controls expiry and scheduling.
- source_event_id is present when deduplicating an external/device source identity.
- correlation_id links an objective/run/operation path; causation_id links the accepted command/event that caused this fact.
- trust is derived from authentication at ingress, never copied uncritically from an untrusted payload.
- classification carries sensitivity and scope for policy enforcement. It does not replace access-control checks.
- payload_digest_sha256 is over the exact canonical UTF-8 JSON bytes of data after RFC 8785 canonicalization.
- data contains the minimum necessary fields. Large content is represented by a governed reference plus digest, not embedded in a general-purpose event.

## 3. Subject convention

Subject pattern: `lia.v1.account.<account_id>.<aggregate_type>.<event_name>`. IDs are UUID text without separators when placed in a NATS token. Event names use lowercase dot-separated nouns/verbs. Account identifiers in subjects are routing metadata, not a secret; credentials, user names, file paths and content never appear in subjects.

Consumers subscribe only to the account subjects authorized for their service identity. The handler independently validates account ownership and the source-domain contract before applying state. An account wildcard is not a substitute for tenant authorization.

The stream wildcard is `lia.v1.account.*.>`. Subject names and ownership are listed in [subjects.yaml](subjects.yaml).

## 4. Outbox, publisher and inbox

1. The domain owner commits its state transition and event_outbox row in one database transaction.
2. An outbox publisher reads pending rows in bounded batches and publishes the envelope to JetStream. It uses event_id as the NATS message deduplication identifier where supported.
3. The publisher marks a row published only after receiving the broker's publish acknowledgement. An uncertain publish is retried with the same event_id.
4. A durable pull consumer receives the event with explicit acknowledgement policy.
5. The consumer validates schema, account, producer trust, aggregate sequence, source event identity and policy.
6. The consumer commits its inbox receipt and resulting domain transition/projection atomically where they share a database transaction. Where they do not share one, a recoverable handoff and idempotent state transition are required.
7. The consumer acknowledges the broker message only after durable acceptance or durable quarantine. A lost acknowledgement may produce redelivery; the inbox dedupe makes it harmless.

JetStream message deduplication and acknowledgement features do not establish exactly-once application effects. The stream is a bounded transport tier; canonical audit/history retention is provided by the owning domain. Stable event IDs, inbox uniqueness and domain idempotency remain mandatory.

## 5. Ordering, duplicates and gaps

- A repeated event_id or source_event_id does not create a second transition.
- Aggregate sequence, not wall-clock time or broker order, establishes canonical order within an aggregate.
- A consumer seeing sequence N+1 before N defers processing and retrieves/reconciles canonical aggregate state or requests replay. It must not invent the missing transition.
- Events from unrelated aggregates may interleave freely.
- Corrections are additional events with provenance; committed event history is not silently rewritten.
- A malformed or semantically incompatible event is durably quarantined with a non-disclosing reason. It is not passed to an LLM as a recovery mechanism.

## 6. Retry and quarantine

Each durable consumer declares:
- accepted event types and schema versions;
- explicit acknowledgement behavior;
- bounded delivery attempts and backoff;
- maximum in-flight messages and processing deadline;
- handling for permanent schema/authentication/policy failures;
- quarantine retention and operator/user recovery path;
- maximum replay window and aggregate-gap policy.

Exact AckWait, MaxDeliver, BackOff, stream max age/bytes and quarantine retention are deployment values to be pinned after P3.5 retention/capacity review and NATS-01 load testing. No infinite retention is implied.

Notification delivery is an independent consumer domain. A notification failure cannot mutate workflow truth; a notification click cannot authorize a consequential operation.

## 7. Event payload restrictions

Do not publish:
- private keys, tokens, connector secrets, recovery codes or credential material;
- raw screen, camera or microphone frames/audio;
- full conversation archives, full prompts or unrestricted memory;
- file contents or provider payloads not required by the event contract;
- model-generated instructions as executable fields.

S2 content requires purpose/scope approval and should normally be referenced, not embedded. S3 secrets are never event payloads. The event owner determines the correct scope, sensitivity and retention class before publication.

## 7.1 Transport retention versus canonical event history

JetStream is the bounded delivery transport, not the permanent event archive. All messages on the initial LIA_EVENTS_V1 stream use the transport_transient retention class and the stream has a finite configured max_age/max_bytes. Numeric values are pinned in the deployment profile after the P3.5 retention/capacity review and NATS-01 load test; unlimited retention is prohibited.

Canonical audit/business history remains with the owning domain under its data-retention policy. If an event must outlive the JetStream transport window, its owner persists the canonical fact in its own governed event/audit store and publishes only the minimum routing payload or an opaque reference. JetStream stream-level retention cannot enforce a different per-message TTL for messages sharing that stream; event types requiring different transport retention must be routed to separately configured streams in a later version, not assumed to inherit per-message deletion.

Retention/deletion work-item events are coordination signals only. A tombstone event is emitted by the domain that owns the affected record. Consumers use owner revisions and tombstone IDs to purge stale projections/indexes/caches; broker expiry alone never constitutes deletion. NATS replay after the retention boundary is serviced from the canonical owner, not from an infinite broker history.

## 8. Minimum conformance tests

NATS-01 covers outbox crash windows, publish acknowledgement loss, duplicate publish, duplicate consumer delivery, aggregate reordering/gaps, consumer crash before/after DB commit, acknowledgement loss, poison event quarantine, account isolation, replay after retention boundary, and operation side-effect non-duplication. Evidence includes broker/consumer config, fixture IDs, DB state, message traces with sensitive data redacted, and repeatable test results.
