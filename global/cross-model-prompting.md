# Cross-Model Prompting Standard

This file captures portable best practices for Claude, OpenAI, and Gemini style agent execution.

## Authoring Rules

- Put critical constraints early and in plain language.
- Use explicit ordered steps over narrative prose.
- Keep tool expectations concrete (`analyze`, `verify`, `report`).
- Separate hard constraints from recommendations.
- Avoid conflicting instructions across files.

## Reliability Patterns

- Ask for missing context only when blocking.
- Prefer observable claims over assumptions.
- Include fallback behavior when a tool/check is unavailable.
- Require final outputs to include verification status.

## Prompt Composition

1. Intent: what role this agent/skill performs.
2. Scope: where it should and should not be used.
3. Process: deterministic ordered workflow.
4. Constraints: MUST DO/MUST NOT DO.
5. Output format: stable template.

## Hallucination Guardrails

- Never invent file paths, symbols, metrics, or command outputs.
- Mark uncertainty explicitly and request evidence when needed.
- For risky recommendations, provide conservative defaults.

## Practicality Defaults

- Bias toward incremental changes.
- Prefer readable patterns over clever but fragile ones.
- Optimize only after identifying bottlenecks or risk hotspots.
