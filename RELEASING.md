# Release Process

This project uses [release-please](https://github.com/googleapis/release-please) for automated releases and changelog generation.

## How It Works

Release-please automatically:
1. Creates release PRs based on conventional commits
2. Updates the `CHANGELOG.md`
3. Bumps the version in `Cargo.toml`
4. Creates GitHub releases
5. Publishes to crates.io (when configured)

## Conventional Commits

Use [conventional commits](https://www.conventionalcommits.org/) format for your commit messages:

### Commit Message Format

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Types

- **feat:** A new feature (bumps minor version)
- **fix:** A bug fix (bumps patch version)
- **docs:** Documentation only changes
- **style:** Changes that don't affect code meaning (formatting, etc.)
- **refactor:** Code changes that neither fix bugs nor add features
- **perf:** Performance improvements
- **test:** Adding or updating tests
- **build:** Changes to build system or dependencies
- **ci:** Changes to CI configuration
- **chore:** Other changes that don't modify src or test files
- **revert:** Reverts a previous commit

### Breaking Changes

To indicate a breaking change (bumps major version):

```
feat!: remove deprecated API method

BREAKING CHANGE: The old API method has been removed. Use the new method instead.
```

Or simply use `!` after the type/scope:
```
feat(api)!: change response format
```

### Examples

```bash
# Feature addition
feat: add support for ESPHome API 1.13
feat(discovery): implement timeout configuration

# Bug fix
fix: correct connection timeout handling
fix(noise): resolve handshake failure on retry

# Documentation
docs: update README with new examples
docs(api): add missing parameter descriptions

# Breaking change
feat!: redesign client builder API

BREAKING CHANGE: The builder pattern has been redesigned. 
Update your code to use the new `.address()` method instead of `.with_addr()`.
```

## Release Workflow

### 1. Develop and Commit

Make your changes and commit using conventional commit messages:

```bash
git add .
git commit -m "feat: add new sensor type support"
git push origin main
```

### 2. Automatic PR Creation

When commits are pushed to `main`, release-please will:
- Analyze commits since the last release
- Determine the next version based on commit types
- Create or update a release PR

### 3. Review and Merge

- Review the automatically generated changelog in the release PR
- Check that the version bump is appropriate
- Merge the PR when ready to release

### 4. Automatic Release

After merging the release PR:
- A GitHub release is automatically created
- The package is published to crates.io (if `CARGO_REGISTRY_TOKEN` is configured)
- Release artifacts are uploaded

## Configuration Files

### `.release-please-manifest.json`
Tracks the current version of the package.

### `release-please-config.json`
Configures how release-please behaves:
- Package name
- Release type (rust)
- Changelog sections
- Version bump strategy

## Setup for Publishing

### 1. Get a crates.io API Token

1. Log in to [crates.io](https://crates.io)
2. Go to Account Settings → API Tokens
3. Create a new token with "Publish new crates" permission

### 2. Add Token to GitHub Secrets

1. Go to your repository Settings → Secrets and variables → Actions
2. Add a new secret named `CARGO_REGISTRY_TOKEN`
3. Paste your crates.io API token

### 3. Verify Cargo.toml Metadata

Ensure your `Cargo.toml` has complete metadata:

```toml
[package]
name = "esphome-client"
version = "0.1.0"  # This will be updated automatically
description = "ESPHome client library for Rust"
license = "MIT"
repository = "https://github.com/daanoz/esphome-client"
homepage = "https://github.com/daanoz/esphome-client"
documentation = "https://docs.rs/esphome-client"
readme = "README.md"
keywords = ["esphome", "iot", "smart-home", "home-automation", "api"]
categories = ["network-programming", "api-bindings"]
```

## Manual Release (Emergency)

If you need to manually trigger a release:

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Update `.release-please-manifest.json`
4. Create a git tag:
   ```bash
   git tag -a v0.2.0 -m "Release v0.2.0"
   git push origin v0.2.0
   ```
5. Publish to crates.io:
   ```bash
   cargo publish
   ```

## Version Strategy

This project follows [Semantic Versioning](https://semver.org/):

- **Major (x.0.0)**: Breaking changes (e.g., `feat!:` or `BREAKING CHANGE:`)
- **Minor (0.x.0)**: New features (e.g., `feat:`)
- **Patch (0.0.x)**: Bug fixes (e.g., `fix:`)

Before 1.0.0:
- Minor version bumps may include breaking changes
- Patch version bumps are for bug fixes and non-breaking features

After 1.0.0:
- Strict semantic versioning applies
- Breaking changes require major version bump

## Troubleshooting

### Release PR Not Created

- Check that commits follow conventional commit format
- Ensure commits are pushed to the `main` branch
- Look at workflow runs in Actions tab for errors

### Release Failed to Publish

- Verify `CARGO_REGISTRY_TOKEN` secret is set correctly
- Check that package name is available on crates.io
- Ensure all required metadata is in `Cargo.toml`
- Verify the package builds successfully: `cargo publish --dry-run`

### Version Conflict

If there's a version mismatch:
1. Update `.release-please-manifest.json` to match `Cargo.toml`
2. Commit and push
3. Release-please will sync on next run

## Best Practices

1. **One logical change per commit** - Makes changelog more readable
2. **Descriptive commit messages** - They become your changelog
3. **Test before merging** - CI should pass before merging release PRs
4. **Review changelogs** - Check generated changelog for accuracy
5. **Document breaking changes** - Include migration guide in commit body

## Resources

- [Conventional Commits](https://www.conventionalcommits.org/)
- [Release Please Documentation](https://github.com/googleapis/release-please)
- [Semantic Versioning](https://semver.org/)
- [Keep a Changelog](https://keepachangelog.com/)
