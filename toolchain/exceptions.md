# Toolchain exceptions

## Node.js 26.13.1 for contract CI

- **Date:** 2026-11-14
- **Scope:** CI validator only; this does not select a coordinator or product runtime.
- **Exception:** Use before the seven-day dependency cool-down expires.
- **Reason:** The Node.js security release published 2026-11-11 patches multiple High-severity HTTP/2/TLS issues affecting the 26.x line. Pinning the previous 26.x binary would leave CI on a known-vulnerable runtime.
- **Primary notice:** https://nodejs.org/en/blog/vulnerability/november-2026-security-releases/
- **Reviewer:** Repository owner review requested in PR #3; approval is pending.
- **Expiry:** Remove this record after 2026-11-18, or immediately on an approved later patch pin.

The branch may run CI to surface issues while this exception awaits review; the exception is not evidence that a production Node baseline has been approved.
