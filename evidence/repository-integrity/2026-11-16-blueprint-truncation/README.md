# Blueprint truncation incident and recovery

**Incident:** Commit `6960fba` replaced `LIA-Project-Blueprint.md` with captured/truncated command output. The affected file contained `Warning: truncated output`, `tokens truncated`, and a `Total output lines:` transcript marker. The repository history contained the intact blueprint at `010202c`.

**Recovery:** The full file body was restored byte-for-byte from `010202c:LIA-Project-Blueprint.md`. Only the later version header and Section 20.14 P5 progress/status text were reapplied from the branch head. Section 20.13 and all earlier blueprint content remain from the intact source commit.

**Baseline comparison:** `origin/main` had 726,042 bytes, 8,903 lines and 696 Markdown headings. Immediately after restoration (before the latest M2 status entry), the recovered P5 branch version had 733,196 bytes, 8,923 lines and 696 headings. The additional 7,154 bytes and 20 lines are the P5 status updates; heading count matches the baseline. No truncation markers remain. At the end of the 16 November recovery task, after its M2 and M3 updates, the working copy measured 734,480 bytes, 8,929 lines and 696 headings; `origin/main` remains at 726,042 bytes, 8,903 lines and 696 headings.

**Prevention:** `scripts/check-blueprint-integrity.py` rejects the known output markers and requires at least 600,000 bytes, 8,000 lines and 650 headings. The contract-hygiene CI workflow runs it on every pull request and push to `main`; [run 43859236182](https://github.com/ranjanssgj/lia/actions/runs/43859236182) passed the integrity check and its regression cases.

**Validation:** Local integrity check and regression cases; the linked CI run passed the same checks. This repository-integrity check does not mark any P4/P5 implementation or conformance gate Passed.
