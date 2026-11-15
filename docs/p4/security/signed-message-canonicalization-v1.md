# Signed-message canonicalization profile (draft)

This profile fixes the bytes signed by P4 `CommandEnvelope` and `OfflineAuthorizationLease`. The protobuf schema is authoritative for field numbers and types; the allowlisted JSON mapping below is the only canonicalization input. Signature and transport-only fields are excluded.

## ProtoJSON mapping

- Encode only the declared allowlisted message fields using their protobuf JSON names. Reject unknown fields, duplicate JSON names, duplicate aliases, invalid enum values, and values outside the protobuf type range before canonicalization.
- `int64`, `uint64`, and fixed-width 64-bit integers are decimal JSON strings, with no leading `+`, no leading zero except `"0"`, and no `-0`. This includes `sessionSequence`, revisions, generations, and epochs.
- `bytes` values are standard padded RFC 4648 base64 strings. Reject URL-safe alphabet, whitespace, missing padding where required, and noncanonical alternate encodings. Hashes and signatures are encoded the same way; the message signature field itself is excluded from the signed object.
- Timestamps use UTC RFC 3339 with `Z`, seconds, and optional fractional seconds of 1–6 digits. Normalize offsets to UTC and trim trailing fractional zeroes. Reject leap seconds and excess precision.
- Enum values use the exact protobuf enum identifiers, not numbers or aliases. Unknown enum names are rejected.
- Omit absent optional fields. Preserve explicit `false`, zero, and empty values only when the schema presence model distinguishes them from absence. Do not synthesize defaults for absent fields. Repeated fields retain order; maps are serialized as JSON objects and RFC 8785 determines key order.
- Strings are valid Unicode scalar sequences; reject invalid UTF-8 and unpaired surrogates. Do not apply Unicode normalization.
- Canonicalize the resulting JSON with RFC 8785 (JCS), UTF-8 encode it, then append it to the domain prefix plus one LF byte: `LIA-COMMAND-V1\n` or `LIA-OFFLINE-LEASE-V1\n`.

These rules are a draft until matched byte-for-byte against generated Rust and TypeScript protobuf implementations. Canonical JSON tests alone do not establish protobuf interoperability.

## Local checks

The crate tests RFC 8785 number formatting, UTF-16 ordering for non-BMP keys, domain separation, payload/key tampering, truncated signatures, and strict Ed25519 rejection of noncanonical signatures. Expiry and replay ordering are state-machine tests in the device store, not signature-verifier properties. Vectors must be generated from the normalized allowlisted object and include expected canonical bytes, signing input, digest, public key, and signature.
