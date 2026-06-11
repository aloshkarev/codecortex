# Expert Agent/Skill Framework

Use this as the shared operating contract for `agents/`, `skills/`, and `instructions/`.

## Design Principles

- Single responsibility per file: one role, one skill, one concern.
- Reuse through references: keep deep details in `skills/*/references/*.md`.
- Deterministic execution: prioritize evidence-driven actions and verifiable outputs.
- Production safety first: include validation and rollback guidance for risky operations.

## Agent Structure Standard

1. Frontmatter: `name`, `description`, `tools`, `model`.
2. Role statement: what the agent owns and what it does not.
3. Decision flow: triage, analysis, execution, verification.
4. Severity model: Critical, Major, Minor, Suggestion.
5. Output contract: fixed report shape for predictable handoffs.
6. Skill bindings: explicit `@skills/.../SKILL.md` references.

## Skill Structure Standard

1. Frontmatter metadata for routing and discovery.
2. Clear role definition and use-cases.
3. Core workflow with ordered checkpoints.
4. Constraints as MUST DO/MUST NOT DO.
5. Output templates for reproducibility.
6. Reference map to domain playbooks and anti-patterns.

## Verification Contract

Every non-trivial recommendation must include one of:

- compile/build check
- test check (unit/integration/e2e as applicable)
- lint/static-analysis check
- runtime/safety check (sanitizers, profilers, or diagnostics)

## Escalation Rules

- Unknown blast radius: trigger impact-analysis first.
- Risky platform changes: require canary/rollback plan.
- Missing context: ask for constraints before irreversible actions.
