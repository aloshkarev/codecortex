# Debugger Triage Checklist

Apply this triage flow before proposing fixes.

1. Classify failure:
   - compile/link/type error
   - runtime crash/panic
   - logical regression
   - performance anomaly
2. Reproduce mentally or via evidence:
   - input/state assumptions
   - control flow path
   - ownership/lifetime/memory/thread behavior
3. Isolate root cause:
   - exact violated contract/invariant
   - triggering condition
4. Propose minimal fix first:
   - targeted patch
   - avoids broad rewrites
5. Add prevention:
   - test/sanitizer/static check
   - guardrails for recurrence
