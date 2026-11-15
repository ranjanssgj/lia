# Test data

Only public, standards-derived or synthetic fixtures belong here. Each vector records its source, purpose, input bytes, expected output, license/source reference and whether it tests a primitive or a LIA protocol profile.

`security/ed25519-rfc8032.json` is an RFC 8032 primitive known-answer vector. It does not prove the LIA signature profile, key lifecycle, envelope canonicalization, target binding or replay protection. `security/jcs-sha256.json` covers only simple object key ordering and SHA-256. Domain-specific SECURITY-01 vectors must be added after the protocol’s exact canonical payload and signature domain are frozen.

Public keys in this directory are fixtures, not credentials. `.gitignore` must not hide any file under `testdata/`, including `.pem`, `.key`, `.db` or `.sqlite` files.
