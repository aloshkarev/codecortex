# Naming Conventions

Use these conventions for consistent discovery and routing.

## Agents

### Coding Domains

Format: `<domain>-code-<role>.md`

- Domains: `rust`, `cpp`, `c`, `typescript`, `python`, `golang`
- Roles: `developer`, `debugger`, `reviewer`, `optimizer`

Examples:

- `rust-code-developer.md`
- `cpp-code-reviewer.md`
- `python-code-optimizer.md`

### Non-Coding/System Domains

Format: `<domain>-<role>.md`

- Domains: `system-architecture`, `techlead`, `staff-engineering`, `devops`
- Roles: `designer`, `reviewer`, `debugger`, `adviser`, `analyzer`, `optimizer`

Examples:

- `system-architecture-designer.md`
- `techlead-reviewer.md`
- `staff-engineering-analyzer.md`

## Skills

Format: `<domain>-<role>` or established specialist name when role is implied.

- Role-oriented: `rust-reviewer`, `golang-debugger`, `devops-optimizer`
- Specialist-oriented (kept for compatibility): `react-expert`, `api-designer`, `test-master`

## Compatibility Rule

When renaming would break existing references, keep compatibility aliases and introduce new canonical names for future additions.
