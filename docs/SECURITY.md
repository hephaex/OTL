# OTL Security Documentation

## Overview

This document describes the security architecture, threat model, and security controls implemented in the OTL (Ontology-based Knowledge System) API.

## Security Architecture

### Authentication & Authorization

| Component | Implementation |
|-----------|----------------|
| Authentication | JWT (JSON Web Tokens) with HMAC-SHA256 |
| Password Hashing | Argon2id (OWASP recommended parameters) |
| Session Management | Stateless JWT with token blacklist |
| Access Control | Role-based (admin, editor, viewer) |

### Token Configuration

| Parameter | Value |
|-----------|-------|
| Access Token Expiry | 1 hour |
| Refresh Token Expiry | 7 days |
| Token Rotation | On refresh |
| Blacklist | In-memory (single instance) |

### Security Headers

All API responses include the following security headers:

| Header | Value | Purpose |
|--------|-------|---------|
| X-Content-Type-Options | nosniff | Prevent MIME sniffing |
| X-Frame-Options | DENY | Prevent clickjacking |
| X-XSS-Protection | 1; mode=block | XSS filtering |
| Strict-Transport-Security | max-age=31536000; includeSubDomains | HTTPS enforcement |
| Content-Security-Policy | default-src 'self' | Resource loading restrictions |
| Referrer-Policy | strict-origin-when-cross-origin | Referrer control |
| Permissions-Policy | geolocation=(), camera=(), microphone=() | Feature restrictions |

## Threat Model

### Assets

1. **User Data**: Credentials, personal information
2. **Documents**: Uploaded files, extracted content
3. **Knowledge Graph**: Entities, relations, ontology
4. **API Keys**: JWT secrets, database credentials

### Threat Actors

| Actor | Motivation | Capability |
|-------|------------|------------|
| External Attackers | Data theft, service disruption | Network access, public exploits |
| Malicious Users | Data exfiltration, privilege escalation | Authenticated access |
| Insider Threats | Data theft, sabotage | Full system access |

### Threats and Mitigations

#### OWASP Top 10 Coverage

| Threat | Mitigation |
|--------|------------|
| A01:2021 Broken Access Control | Role-based access control, JWT validation |
| A02:2021 Cryptographic Failures | Argon2id password hashing, secure JWT |
| A03:2021 Injection | Parameterized queries (sqlx), input validation |
| A04:2021 Insecure Design | Security review, threat modeling |
| A05:2021 Security Misconfiguration | Secure defaults, CORS restrictions |
| A06:2021 Vulnerable Components | cargo-audit, cargo-deny, dependency scanning |
| A07:2021 Auth Failures | Rate limiting (planned), account lockout |
| A08:2021 Data Integrity Failures | Magic bytes validation, file type checking |
| A09:2021 Logging Failures | Security audit logging |
| A10:2021 SSRF | URL validation, restricted network access |

### Attack Surface

```
┌─────────────────────────────────────────────────────────────┐
│                     External Network                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    Load Balancer                     │   │
│  │              (HTTPS termination)                     │   │
│  └───────────────────────┬─────────────────────────────┘   │
│                          │                                   │
│  ┌───────────────────────▼─────────────────────────────┐   │
│  │                    OTL API                           │   │
│  │  ┌──────────────────────────────────────────────┐   │   │
│  │  │           Security Middleware                 │   │   │
│  │  │  • Security Headers                          │   │   │
│  │  │  • CORS Validation                           │   │   │
│  │  │  • Rate Limiting (planned)                   │   │   │
│  │  │  • Request Size Limits                       │   │   │
│  │  └──────────────────────────────────────────────┘   │   │
│  │  ┌──────────────────────────────────────────────┐   │   │
│  │  │           Auth Middleware                     │   │   │
│  │  │  • JWT Validation                            │   │   │
│  │  │  • Token Blacklist                           │   │   │
│  │  │  • Role-based Access                         │   │   │
│  │  └──────────────────────────────────────────────┘   │   │
│  │  ┌──────────────────────────────────────────────┐   │   │
│  │  │           Audit Logging                       │   │   │
│  │  │  • Auth Events                               │   │   │
│  │  │  • Access Decisions                          │   │   │
│  │  │  • Security Events                           │   │   │
│  │  └──────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────┘   │
│                          │                                   │
│  ┌───────────────────────▼─────────────────────────────┐   │
│  │              Internal Services                       │   │
│  │  • PostgreSQL (user data, metadata)                 │   │
│  │  • SurrealDB (knowledge graph)                      │   │
│  │  • Qdrant (vector embeddings)                       │   │
│  │  • LLM Provider (OpenAI/Ollama)                     │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Security Controls

### Input Validation

| Input | Validation |
|-------|------------|
| File Uploads | Size limit (50MB), magic bytes validation |
| Passwords | Minimum 8 chars, complexity requirements |
| Email | Format validation |
| Query Parameters | Type checking, sanitization |

### Audit Logging

All security-relevant events are logged with:
- Timestamp
- Event type
- User ID (if authenticated)
- IP address
- User agent
- Event-specific details

Events logged:
- Login attempts (success/failure)
- Registration attempts
- Logout events
- Token refresh
- Password changes
- Access denied events
- Invalid token attempts
- Account lockouts

### Vulnerability Management

| Tool | Purpose |
|------|---------|
| cargo-audit | Rust dependency CVE scanning |
| cargo-deny | License and dependency policy |
| detect-secrets | Secret detection in code |
| pre-commit hooks | Automated security checks |

### Security Testing

```bash
# Run security audit
cargo audit

# Run dependency policy check
cargo deny check

# Run clippy with security lints
cargo clippy --all-targets -- -D warnings

# Run all pre-commit hooks
pre-commit run --all-files
```

## Incident Response

### Security Issue Reporting

Report security vulnerabilities to: hephaex@gmail.com

Please include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response Timeline

| Severity | Initial Response | Fix Timeline |
|----------|-----------------|--------------|
| Critical | 24 hours | 48 hours |
| High | 48 hours | 1 week |
| Medium | 1 week | 2 weeks |
| Low | 2 weeks | Next release |

## Compliance

### Security Standards

- OWASP Top 10 (2021)
- CWE/SANS Top 25
- NIST Cybersecurity Framework

### Data Protection

- No PII stored without encryption
- Passwords hashed with Argon2id
- Audit logs do not contain sensitive data
- Token blacklist for session invalidation

## Future Enhancements

1. **Rate Limiting**: Per-endpoint rate limits using tower_governor
2. **MFA Support**: Time-based one-time passwords (TOTP)
3. **OAuth2/OIDC**: External identity provider integration
4. **Secret Management**: HashiCorp Vault integration
5. **WAF Integration**: Web Application Firewall rules
6. **Penetration Testing**: Regular security assessments
