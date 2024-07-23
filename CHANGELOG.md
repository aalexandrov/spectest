# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog][keepachangelog], and this project
adheres to [Semantic Versioning][semver].

## [Unreleased]

### Added

- Add `async` versions of `Handler`, `run`, `process` and `rewrite` and teach
  `glob_test` to generate correct tests for an annotated `async` function.

### Changed

- Teach `(async_)process` and `(async_)rewrite` to skip `Example` sections whose
  name ends with `(ignored)`.
- Introduce file locking for all spectest files. This allows users to run tests
  with `REWRITE_SPECS=1 cargo tests` without worrying about write conflicts in
  cases where multiple tests interpret the same spectest file.

### Removed

<!-- TODO -->

## [0.1.1] - 2024-06-26

No API changes, this release only fixes linter errors reported by `clippy`.

## [0.1.0] - 2024-06-24

### Added

- Initial `Handler` API version.
- Limited support for Markdown syntax.
- Initial version of the `glob_test` macro.

[keepachangelog]: https://keepachangelog.com/en/1.1.0/
[semver]: https://semver.org/spec/v2.0.0.html
[unreleased]: https://github.com/aalexandrov/spectest/compare/v0.1.1...dev
[0.1.1]: https://github.com/aalexandrov/spectest/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/aalexandrov/spectest/tree/v0.1.0
