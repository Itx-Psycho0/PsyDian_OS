# CI/CD

## 1. Purpose

The CI/CD strategy for Psydian is intended to automate validation of the codebase and provide a consistent process for building, testing, and preparing the project for demonstration.

Because Psydian contains both a bare-metal Rust kernel and host-side components such as the AI Bridge, the CI/CD pipeline must validate both environments independently and together.

The primary goals are:

- Detect compilation failures early.
- Validate kernel code changes.
- Validate host-side AI Bridge code.
- Run automated tests.
- Perform formatting and lint checks.
- Detect dependency and security issues.
- Build the bootable Psydian image.
- Run appropriate integration/smoke checks.
- Prevent broken code from being merged into the stable branch.
- Maintain reproducible build configuration.

---

# 2. CI/CD Scope

The CI/CD pipeline covers:

```text
Source Code
    ↓
Formatting / Linting
    ↓
Unit Tests
    ↓
Kernel Build
    ↓
Host-side Build / Tests
    ↓
Integration Checks
    ↓
Security / Dependency Checks
    ↓
Boot Image Build
    ↓
Smoke Test
    ↓
Artifact Generation
```

The pipeline is primarily intended for development and project demonstration rather than production infrastructure.

---

# 3. CI/CD Architecture

```text
Developer
    ↓
Feature Branch
    ↓
Pull Request
    ↓
Continuous Integration
    ├── Format Check
    ├── Rust Lint
    ├── Kernel Compile
    ├── Host Tests
    ├── Protocol Tests
    ├── AI Bridge Tests
    ├── Security Checks
    └── Boot Image Build
            ↓
        Smoke Tests
            ↓
        Pull Request Review
            ↓
          main
            ↓
      Build Release Artifact
```

---

# 4. Development Environments

Psydian should distinguish between development and stable code.

```text
Development
    ↓
Feature Branch
    ↓
Pull Request
    ↓
CI Validation
    ↓
Code Review
    ↓
main
```

The project may maintain the following environments/states:

- Local development.
- Continuous integration.
- Stable/main branch.
- Demo/release build.

A separate production environment is not required for the MVP.

---

# 5. Branching Strategy

Recommended branches:

```text
main
develop
feature/*
```

## `main`

Purpose:

- Stable code.
- Demonstration-ready state.
- Tested baseline.

Changes should enter `main` only after required checks pass.

## `develop`

Optional integration branch for combining multiple features before they are promoted to `main`.

For a two-person team, this branch is optional.

## `feature/*`

Used for individual work.

Examples:

```text
feature/kernel-memory
feature/keyboard
feature/shell
feature/diagnostics
feature/ai-bridge
feature/voice
```

Feature branches should be short-lived where practical.

---

# 6. Pull Request Process

Each pull request should contain:

- Problem.
- Solution.
- Testing performed.
- Relevant screenshots or terminal output.
- Risks or known limitations.

Example:

```text
## Problem
Kernel panic diagnostics did not contain subsystem information.

## Solution
Added structured subsystem metadata to diagnostic records.

## Testing
- cargo test
- kernel build
- QEMU smoke test
- controlled panic test

## Risks
Diagnostic structure changed and AI Bridge parser was updated.

## Evidence
Attached QEMU output and test results.
```

---

# 7. Continuous Integration Pipeline

The main CI pipeline should follow:

```text
Pull Request
    ↓
Checkout Repository
    ↓
Install/Use Required Toolchain
    ↓
Formatting Check
    ↓
Lint / Static Checks
    ↓
Unit Tests
    ↓
Kernel Build
    ↓
Host-side Tests
    ↓
Protocol Tests
    ↓
AI Bridge Tests
    ↓
Security / Dependency Checks
    ↓
Boot Image Build
    ↓
Smoke Tests
    ↓
Artifact Upload
```

---

# 8. Toolchain Reproducibility

The project should maintain its working Rust toolchain configuration in version control.

The repository should document:

```text
Rust nightly
rust-src
llvm-tools-preview
x86_64-unknown-none
QEMU version
```

The CI environment should use the same documented toolchain configuration as local development wherever possible.

This reduces:

- Compiler mismatches.
- Dependency inconsistencies.
- Build-only-on-one-machine problems.

---

# 9. Formatting

Rust source should be checked using the project's configured formatter.

Typical check:

```bash
cargo fmt --all -- --check
```

The CI pipeline should fail if formatting does not match the repository standard.

Developers can fix formatting with:

```bash
cargo fmt --all
```

---

# 10. Linting and Static Analysis

The CI pipeline should perform appropriate Rust linting.

For example:

```bash
cargo clippy --workspace --all-targets
```

Kernel-specific targets may require target-aware invocation depending on the final workspace configuration.

Warnings originating from external dependencies should be distinguished from project-code failures.

The project should avoid weakening lint settings simply to hide actual errors in Psydian code.

---

# 11. Unit Testing

Host-side logic that does not require the booted kernel should be covered by normal Rust tests.

Typical command:

```bash
cargo test
```

Potential targets:

- Command parsing.
- Argument validation.
- Diagnostic serialization.
- Diagnostic parsing.
- Protocol validation.
- AI response parsing.
- AI safety checks.
- Configuration handling.
- Utility functions.

---

# 12. Kernel Build Validation

The CI pipeline shall verify that the kernel successfully compiles for the intended target.

Target:

```text
x86_64-unknown-none
```

The pipeline should confirm:

- Kernel source compiles.
- Bare-metal target is available.
- Required Rust components are available.
- Bootloader integration remains functional.

A build failure should block the pull request.

---

# 13. Boot Image Validation

The CI pipeline should build the bootable Psydian image.

Conceptually:

```text
Kernel
    ↓
Bootloader
    ↓
BIOS Image
    ↓
Smoke Test
```

The generated artifact should be retained by CI when useful for debugging or demonstration.

Expected artifact:

```text
psydian-bios.img
```

---

# 14. QEMU Smoke Testing

A smoke test verifies that the newly built image can start.

The minimum smoke test should verify:

```text
QEMU
 ↓
Bootloader
 ↓
Kernel Entry
 ↓
Expected Kernel Output
```

For the current implementation, an example expected output is:

```text
[INFO] Psydian kernel booted.
```

A failure to reach this output should fail the smoke test.

---

# 15. Kernel Runtime Testing

Where practical, CI should execute QEMU with a bounded runtime.

Example conceptual flow:

```text
Start QEMU
    ↓
Boot Image
    ↓
Capture Serial Output
    ↓
Wait for Expected Marker
    ↓
PASS / FAIL
    ↓
Terminate QEMU
```

A timeout should be used so a kernel infinite loop or deadlock does not cause the CI job to run indefinitely.

---

# 16. Communication Protocol Testing

The kernel/host diagnostic protocol should be tested independently from the external AI provider.

Tests should verify:

- Valid messages.
- Invalid messages.
- Unsupported versions.
- Missing fields.
- Oversized messages.
- Unknown message types.
- Serialization.
- Deserialization.
- Error handling.

Example:

```text
Valid DIAGNOSTIC
    ↓
Accepted

Malformed DIAGNOSTIC
    ↓
Rejected
```

---

# 17. AI Bridge CI Testing

The AI Bridge should be tested without making every CI run dependent on a live external AI service.

Use:

- Mock responses.
- Recorded responses.
- Fixture data.
- Provider-independent tests.

Test:

- Request creation.
- Protocol parsing.
- Prompt/context generation.
- Response parsing.
- Schema validation.
- Safety checks.
- Timeout handling.
- Error handling.

---

# 18. External AI API Testing

Live external AI integration tests may be separated from ordinary CI because they can introduce:

- API costs.
- Network instability.
- Rate limits.
- Credential-management issues.
- Provider downtime.

A safer structure is:

```text
Normal CI
    ↓
Mock AI Provider
    ↓
Deterministic Tests
```

and optionally:

```text
Manual / Scheduled Integration Test
    ↓
Configured External AI Provider
```

This prevents the stability of the build pipeline from depending on an external API.

---

# 19. Security and Dependency Checks

The CI pipeline should include security-related checks appropriate to the project.

Examples:

- Dependency vulnerability scanning.
- Secret detection.
- Repository secret checks.
- Review of newly introduced dependencies.
- Static analysis where appropriate.

A pull request should be blocked when:

- Required security checks fail.
- Secrets are detected.
- A critical dependency issue requiring immediate action is identified.

---

# 20. Secret Management in CI

CI secrets must be stored using the CI platform's secret-management facility.

Examples:

```text
AI_API_KEY
AI_ENDPOINT
OTHER_PROVIDER_CREDENTIALS
```

Secrets must not appear in:

```text
Cargo.toml
source code
Git history
workflow files
serial output
build artifacts
logs
```

The kernel build itself should not require the external AI API key.

Only the host-side AI integration test or deployment environment should receive it.

---

# 21. AI Evaluation in CI

Deterministic AI evaluation should use a fixed evaluation set.

Example:

```text
Diagnostic Fixture
        ↓
Expected Evidence
        ↓
Expected/Allowed Response Properties
        ↓
AI Bridge Test
        ↓
Validation Result
```

CI can verify:

- Response schema validity.
- Required fields.
- Safety flags.
- Unsupported command detection.
- Known fixture behavior.

Full semantic evaluation can be performed separately when it requires a live model.

---

# 22. Integration Testing

Integration tests should verify communication between major components.

Important paths:

### Kernel → Serial

```text
Kernel
 ↓
Logger
 ↓
UART
 ↓
QEMU
 ↓
Host Output
```

### Kernel → AI Bridge

```text
Kernel
 ↓
Diagnostic Protocol
 ↓
AI Bridge
```

### AI Bridge → AI Provider

```text
AI Bridge
 ↓
HTTPS
 ↓
AI Provider
```

### Complete AI Path

```text
Kernel
 ↓
Diagnostic
 ↓
AI Bridge
 ↓
AI Provider
 ↓
Validated Response
 ↓
Psydian
```

---

# 23. Continuous Delivery

For an academic MVP, continuous delivery should mean:

- A validated build can produce a bootable artifact.
- A stable `main` branch remains demonstration-ready.
- Release artifacts can be generated reproducibly.

A full production deployment pipeline is not required.

---

# 24. Build Artifacts

Useful CI artifacts include:

```text
psydian-bios.img
kernel binary / ELF
test logs
QEMU serial output
AI Bridge test results
coverage reports where available
security scan results
```

Artifacts should be generated only from successful builds.

---

# 25. Release Process

A demonstration/release build can follow:

```text
Feature Branches
      ↓
Pull Requests
      ↓
CI
      ↓
Code Review
      ↓
main
      ↓
Release Build
      ↓
Bootable Image
      ↓
Final Smoke Test
      ↓
Demo Artifact
```

A release tag may be created for major milestones.

Examples:

```text
v0.1.0-kernel-boot
v0.2.0-shell
v0.3.0-diagnostics
v0.4.0-ai-bridge
v1.0.0-mvp
```

The final naming scheme can be simplified if desired.

---

# 26. Quality Gates

A pull request should not merge when required checks fail.

Minimum quality gates:

```text
Formatting
   ✓

Compilation
   ✓

Required Tests
   ✓

Kernel Build
   ✓

Boot Image Build
   ✓

Smoke Test
   ✓

Security / Secret Check
   ✓
```

Optional checks should be clearly distinguished from blocking checks.

---

# 27. Definition of Done

A feature is considered complete when it is:

- Implemented.
- Tested.
- Documented.
- Integrated with the required subsystem.
- Covered by relevant error handling.
- Reviewed for security implications.
- Demonstrable in the QEMU environment where applicable.

---

# 28. CI/CD Failure Handling

When a CI job fails:

```text
Failure
  ↓
Identify Stage
  ↓
Inspect Logs
  ↓
Reproduce Locally
  ↓
Fix
  ↓
Push Update
  ↓
CI Re-run
```

The team should avoid bypassing a failing quality gate simply to merge a feature.

---

# 29. QEMU CI Considerations

Running QEMU in CI can require additional runner configuration.

The project should initially prioritize:

1. Build validation.
2. Host-side tests.
3. Boot-image generation.
4. QEMU smoke testing where the CI environment supports it.

If a CI runner cannot reliably launch QEMU, boot testing can temporarily remain a controlled local validation step while preserving automated kernel compilation and artifact generation.

---

# 30. GitHub Actions Structure

The project can use GitHub Actions with workflows such as:

```text
.github/
└── workflows/
    ├── ci.yml
    ├── security.yml
    └── release.yml
```

### `ci.yml`

Responsible for:

- Formatting.
- Linting.
- Unit tests.
- Kernel build.
- Boot-image build.
- Integration tests.
- Smoke tests where supported.

### `security.yml`

Responsible for:

- Secret scanning.
- Dependency checks.
- Security-related static checks.

### `release.yml`

Responsible for:

- Stable build.
- Bootable image generation.
- Artifact packaging.
- Optional GitHub release creation.

---

# 31. Example CI Workflow

Conceptually:

```yaml
name: Psydian CI

on:
  push:
    branches:
      - main
      - develop
  pull_request:

jobs:
  validate:
    runs-on: ubuntu-latest

    steps:
      - Checkout repository

      - Setup Rust nightly

      - Check formatting

      - Run lint checks

      - Run unit tests

      - Build kernel

      - Build bootable image

      - Run protocol/AI Bridge tests

      - Run security checks

      - Run QEMU smoke test where supported

      - Upload artifacts
```

The final workflow syntax should be adjusted to the actual project structure and CI runner capabilities.

---

# 32. Local vs CI Responsibilities

## Local Development

Developers should run:

```text
Formatting
Linting
Unit Tests
Kernel Build
QEMU
Manual Integration Tests
```

before opening a pull request.

## CI

CI should provide:

```text
Clean-environment verification
Automated tests
Build validation
Security checks
Boot-image generation
Smoke testing
Artifacts
```

The goal is not to make developers wait for CI to discover obvious local failures.

---

# 33. CI/CD Metrics

Useful project-level metrics include:

- CI success rate.
- Build duration.
- Test pass rate.
- Number of failed builds.
- Number of security findings.
- QEMU smoke-test success rate.
- AI Bridge test success rate.
- Time from pull request to passing build.

These metrics are primarily for engineering visibility and are not intended as production SLOs.

---

# 34. Documentation Integration

CI/CD configuration and usage should be documented in the repository.

Documentation should explain:

- Required toolchain.
- Local test commands.
- Build commands.
- QEMU commands.
- CI workflow purpose.
- Secret configuration.
- Release process.
- Troubleshooting steps.

This allows a new contributor to reproduce the project's validation process.

---

# 35. MVP CI/CD Scope

## Required

- Git-based workflow.
- Feature branches.
- Pull requests.
- Rust formatting check.
- Rust linting.
- Unit tests.
- Kernel compilation.
- Bootable image generation.
- Protocol/AI Bridge tests.
- Secret/dependency checks.
- QEMU smoke testing where supported.
- CI artifacts.
- Stable `main` branch.

## Optional

- Automated QEMU integration environment.
- Coverage reporting.
- Scheduled live AI-provider tests.
- Automated releases.
- Performance regression tracking.
- Advanced artifact signing.
- Multi-platform CI runners.

Optional CI features should not delay the kernel, shell, diagnostics, or AI MVP.

---

# 36. Final CI/CD Flow

```text
Developer
    ↓
feature/*
    ↓
Commit
    ↓
Pull Request
    ↓
CI
    ├── Format
    ├── Lint
    ├── Unit Tests
    ├── Kernel Build
    ├── Protocol Tests
    ├── AI Bridge Tests
    ├── Security Checks
    ├── Boot Image Build
    └── QEMU Smoke Test
            ↓
       Review / Approval
            ↓
          main
            ↓
      Stable Build
            ↓
     Bootable Artifact
            ↓
        Demonstration
```

---

# 37. CI/CD Principles

### Reproducible

The same repository and toolchain configuration should produce the same build process.

### Automated

Repeatable validation should be handled by CI rather than manual checking alone.

### Fast Feedback

Failures should be detected as early as possible.

### Secure

Secrets and security checks must be handled correctly.

### Non-Blocking Core Development

External AI-provider availability must not unnecessarily prevent kernel development and ordinary CI.

### Demonstrable

The stable branch should always aim to contain a build that can be demonstrated in the supported QEMU environment.

