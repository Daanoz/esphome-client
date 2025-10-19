# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1](https://github.com/Daanoz/esphome-client/compare/v0.1.0...v0.1.1) (2025-10-19)


### Documentation

* fix docs.rs build ([#3](https://github.com/Daanoz/esphome-client/issues/3)) ([afb65d0](https://github.com/Daanoz/esphome-client/commit/afb65d031e8e4631f52737cbfce0c4f24b88981d))

## [Unreleased]

### Added
- Initial release of esphome-client
- Support for multiple ESPHome API versions (1.8, 1.9, 1.10, 1.12)
- Noise protocol encryption support
- Plain text communication support
- mDNS device discovery (with `discovery` feature)
- Comprehensive error types with detailed context
- Stream management with automatic ping handling
- Builder pattern for client configuration
- Example projects demonstrating various use cases
- Full CI/CD pipeline with testing and security auditing

### Features
- Async/await support using Tokio
- Protocol buffer message handling
- Automatic connection setup and authentication
- State subscription and monitoring
- Entity discovery and control
- Connection pooling ready architecture

### Documentation
- Comprehensive README with usage examples
- API documentation
- Contributing guidelines
- Security policy
- Code of conduct
- Multiple example applications

## [0.1.0] - Initial Development

*This changelog will be automatically updated by release-please based on conventional commits.*
