# Review Severity Rubric

Use this shared severity mapping across reviewers and tech-lead/staff roles.

## Critical

- Data loss, security vulnerability, UB/memory corruption, production outage risk.
- Must be fixed before merge/release.

## Major

- High likelihood of correctness bugs, scalability risks, or broken contracts.
- Should be fixed before merge unless explicitly waived with mitigation.

## Minor

- Maintainability, readability, consistency, moderate tech debt.
- Can merge with planned follow-up.

## Suggestion

- Optional improvement with measurable upside.
- No merge blocking impact.

## Required Review Output

1. Finding with severity label.
2. Evidence (code path, behavior, or check result).
3. Actionable fix or mitigation.
4. Verification method.
