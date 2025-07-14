# Security Policy

## Supported Versions

We actively support the following versions of esphome-client with security updates:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security vulnerability in esphome-client, please help us by reporting it responsibly.

### How to Report

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please send fill in the [form](https://forms.gle/LmY3GVoFAXSuJQYN9) with the following information:

- **Subject Line**: "Security Vulnerability in esphome-client"
- **Description**: A detailed description of the vulnerability
- **Steps to Reproduce**: Clear steps to reproduce the issue
- **Impact**: Description of the potential impact
- **Affected Versions**: Which versions are affected
- **Suggested Fix**: If you have suggestions for fixing the issue

### What to Expect

- **Acknowledgment**: We will acknowledge receipt of your vulnerability report within 5 business days
- **Regular Updates**: We will try to keep you informed of our progress
- **Resolution Timeline**: We aim to resolve critical vulnerabilities within 90 days
- **Credit**: We will credit you for the discovery (unless you prefer to remain anonymous)

### Responsible Disclosure

We kindly ask that you:

- Give us reasonable time to investigate and fix the issue before public disclosure
- Avoid accessing, modifying, or deleting data that doesn't belong to you
- Don't perform actions that could harm the reliability or integrity of our services
- Don't use social engineering, physical attacks, or attacks against third parties

### Security Best Practices

When using esphome-client:

#### Secure API Keys
- **Never commit API keys to version control**
- Use environment variables or secure configuration files
- Rotate keys regularly
- Use unique keys per device when possible

#### Network Security
- Use encrypted connections (Noise protocol) when available
- Validate server certificates in production
- Consider using VPNs or network isolation for IoT devices
- Monitor network traffic for unusual patterns

#### Input Validation
- Validate all data received from ESPHome devices
- Sanitize data before logging or displaying
- Be cautious with device names and other user-controllable fields

#### Dependency Management
- Keep dependencies up to date
- Regularly audit dependencies for vulnerabilities
- Use `cargo audit` to check for known vulnerabilities

### Vulnerability Categories

We are particularly interested in vulnerabilities related to:

- **Authentication bypass**: Issues with API key validation
- **Code injection**: Malicious data from devices causing code execution
- **Denial of service**: Issues that could crash or hang the client
- **Information disclosure**: Unintended exposure of sensitive data
- **Cryptographic issues**: Problems with the Noise protocol implementation
- **Dependency vulnerabilities**: Issues in third-party crates

### Security Features

esphome-client includes several security features:

- **Memory safety**: Written in Rust, preventing many common vulnerabilities
- **No unsafe code**: The codebase forbids unsafe code blocks
- **Input validation**: Protocol messages are validated using protobuf
- **Encrypted communication**: Support for Noise protocol encryption
- **Dependency auditing**: Regular security audits of dependencies

### Scope

This security policy covers:

- The esphome-client library code
- Build scripts and configuration
- Documentation that could affect security
- Dependencies when they affect esphome-client security

This policy does not cover:

- ESPHome firmware vulnerabilities (report to ESPHome project)
- Network infrastructure vulnerabilities
- Operating system or hardware vulnerabilities
- Vulnerabilities in applications using esphome-client (unless caused by the library)

Thank you for helping keep esphome-client and its users safe! 🔒