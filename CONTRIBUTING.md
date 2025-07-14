# Contributing to esphome-client

Thank you for your interest in contributing to esphome-client! Contributions are welcome from everyone.

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally
3. Create a new branch for your feature or bug fix
4. Make your changes
5. Test your changes thoroughly
6. Submit a pull request

## Development Setup

### Prerequisites

- Rust 1.70.0 or later
- Git

### Building the Project

```bash
git clone https://github.com/yourusername/esphome-client.git
cd esphome-client
cargo build
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with required features
cargo test --features "discovery api-latest"

# Run integration tests
cargo test --test noise
cargo test --test plain
```

### Code Quality

We maintain high code quality standards:

```bash
# Format code
cargo fmt

# Check for linting issues
cargo clippy -- -D warnings

# Check documentation
cargo doc --no-deps --open
```

## Contributing Guidelines

### Code Style

- Follow Rust's official style guidelines
- Use `cargo fmt` to format your code
- Ensure `cargo clippy` passes without warnings
- Write clear, concise commit messages in [conventional commits format](#commit-message-format)

### Documentation

- Document all public APIs
- Include examples in documentation when helpful
- Update the README if you're adding new features
- Add inline comments for complex logic

### Testing

- Add tests for new functionality
- Ensure existing tests continue to pass
- Include integration tests for new features
- Test with different ESPHome API versions when relevant

### Pull Request Process

1. **Create an Issue**: For significant changes, please create an issue first to discuss the proposed changes
2. **Branch Naming**: Use descriptive branch names (e.g., `feature/add-discovery`, `fix/connection-timeout`)
3. **Commit Messages**: Write clear commit messages following conventional commits format
4. **Testing**: Ensure all tests pass and add new tests as needed
5. **Documentation**: Update documentation for any public API changes
6. **Review**: Be responsive to feedback during the review process

### Commit Message Format

We use conventional commits:

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: A new feature
- `fix`: A bug fix
- `docs`: Documentation only changes
- `style`: Changes that do not affect the meaning of the code
- `refactor`: A code change that neither fixes a bug nor adds a feature
- `test`: Adding missing tests or correcting existing tests
- `chore`: Changes to the build process or auxiliary tools

## Supported ESPHome Versions

We aim to support multiple ESPHome API versions. When contributing:

- Test against the default API version (latest)
- Consider compatibility with older versions when possible
- Update version-specific code in the appropriate feature flags

## Reporting Issues

When reporting issues, please include:

- ESPHome version you're connecting to
- Rust version
- Operating system
- Minimal code example that reproduces the issue
- Error messages and stack traces
- Any relevant logs

## Questions?

If you have questions about contributing, please:

1. Check existing issues and discussions
2. Create a new issue with the "question" label
3. Join discussions in existing issues

Thank you for helping make esphome-client better! 🚀