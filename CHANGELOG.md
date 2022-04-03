# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!--
## [Unreleased]

### Added
### Changed
### Fixed
### Dependencies

-->

## [0.0.1] - 2022-04-03

First public release, and coincides with the compiler source code being made public.

Most of the important functionality _exists_ but is **very rudimentary**.

Namely, there is:

- A parser, typechecker, and linter for an initial module syntax. The expression language is _very_ limited at this stage.
- A JavaScript code generator.
- A build system based on [Ninja](https://ninja-build.org/).
- A basic package manager, based on package sets and local package directories.
- A fairly naive code formatter.
- A language server with `documentFormattingProvider` and `semanticTokensProvider` capabilities. Which are enough to power a basic VS Code extension.
