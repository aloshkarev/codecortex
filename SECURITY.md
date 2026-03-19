# Security Policy

## Supported Versions

We release patches for security vulnerabilities for the latest minor release.
Older versions may receive updates on a best-effort basis.

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly.

**Do not** open a public issue for security vulnerabilities.

### How to report

1. **Email** the maintainer: [mailbox@aloshkarev.com](mailbox@aloshkarev.com)
2. **Include**:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### What to expect

- We will acknowledge receipt within a reasonable timeframe
- We will work with you to understand and validate the report
- We will keep you informed of progress and any fix
- We will credit you in the advisory (unless you prefer anonymity)

### Disclosure

We aim to fix critical vulnerabilities promptly and will coordinate disclosure with you. Public disclosure will typically occur after a patch is available.

## Security Considerations

- **Credentials**: Never commit secrets, tokens, or production credentials. Use environment variables or secure config.
- **Input validation**: All external input is validated and sanitized.
- **Dependencies**: We use `cargo audit` and keep dependencies up to date. Run `cargo audit` locally to check for known vulnerabilities.
