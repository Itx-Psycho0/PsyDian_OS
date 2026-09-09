# GitHub Repository Structure

## 1. Purpose

The Psydian repository should keep source code, documentation, tests, build configuration, scripts, and project artifacts organized so that both team members can understand and reproduce the project.

The repository structure follows the major architectural boundaries of Psydian:

```text
Kernel
Host-side AI Bridge
Documentation
Tests
Build / Tooling
```

The structure should remain simple enough for a two-student, 8-week project while still allowing future expansion.

---

# 2. Recommended Repository Structure

```text
PsyDian_OS/
├── .cargo/
│   └── config.toml
│
├── .github/
│   └── workflows/
│       ├── ci.yml
│       ├── security.yml
│       └── release.yml
│
├── Docs/
│   ├── 01-project-overview.md
│   ├── 02-brd.md
│   ├── 03-prd.md
│   ├── 04-ux-requirements.md
│   ├── 05-trd.md
│   ├── 06-hld.md
│   ├── 07-database-data-design.md
│   ├── 08-api-specification.md
│   ├── 09-lld.md
│   ├── 10-genai-architecture.md
│   ├── 11-security-design.md
│   ├── 12-testing-strategy.md
│   ├── 13-ci-cd.md
│   ├── 14-observability.md
│   ├── 15-deployment-architecture.md
│   ├── 16-cost-analysis.md
│   ├── 17-roadmap.md
│   ├── 18-team-responsibilities.md
│   ├── 19-github-repository-structure.md
│   ├── 20-readme-specification.md
│   ├── 21-adrs.md
│   ├── 22-traceability.md
│   ├── 23-interview-preparation.md
│   ├── 24-viva-project-defense.md
│   └── 25-project-score.md
│
├── kernel/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src/
│       ├── main.rs
│       ├── boot/
│       ├── memory/
│       ├── interrupts/
│       ├── input/
│       ├── serial/
│       ├── logger/
│       ├── shell/
│       ├── diagnostics/
│       └── panic/
│
├── ai-bridge/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src/
│       ├── main.rs
│       ├── protocol/
│       ├── diagnostics/
│       ├── context/
│       ├── prompt/
│       ├── provider/
│       ├── validation/
│       ├── safety/
│       ├── config/
│       └── telemetry/
│
├── tests/
│   ├── kernel/
│   ├── shell/
│   ├── protocol/
│   ├── ai/
│   ├── security/
│   └── integration/
│
├── scripts/
│   ├── build.sh
│   ├── run-qemu.sh
│   ├── test.sh
│   └── clean.sh
│
├── artifacts/
│   └── .gitkeep
│
├── assets/
│   └── diagrams/
│
├── build.rs
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── README.md
├── CONTRIBUTING.md
├── LICENSE
└── .gitignore
```

The exact directories may evolve during implementation. The structure above represents the intended logical organization rather than a requirement that every directory must exist immediately.

---

# 3. Root Directory

The repository root contains project-wide configuration and documentation.

```text
PsyDian_OS/
├── Cargo.toml
├── Cargo.lock
├── build.rs
├── rust-toolchain.toml
├── README.md
└── .gitignore
```

## Responsibilities

### `Cargo.toml`

Defines the root package/workspace configuration and shared project dependencies where applicable.

### `Cargo.lock`

Records dependency versions used by the project.

### `build.rs`

Provides project-level build automation required to combine/build the kernel and bootable image.

### `rust-toolchain.toml`

Defines the project's Rust toolchain requirements.

The repository should document:

```text
Rust nightly
rust-src
llvm-tools-preview
x86_64-unknown-none
```

### `README.md`

Provides the entry point for contributors, evaluators, and mentors.

It should contain:

- Project overview.
- Architecture summary.
- Repository structure.
- Prerequisites.
- Build instructions.
- QEMU run instructions.
- AI Bridge setup.
- Testing instructions.
- Security notes.
- Team information.
- Current project status.

### `.gitignore`

Must exclude generated files, local secrets, and build artifacts that should not be committed.

Examples:

```text
target/
.env
*.log
generated artifacts
local API credentials
temporary files
```

---

# 4. Documentation Directory

All formal project documentation should remain under:

```text
Docs/
```

The documentation sequence follows the mentor-provided complete-documentation structure.

Recommended files:

```text
Docs/
├── 01-project-overview.md
├── 02-brd.md
├── 03-prd.md
├── 04-ux-requirements.md
├── 05-trd.md
├── 06-hld.md
├── 07-database-data-design.md
├── 08-api-specification.md
├── 09-lld.md
├── 10-genai-architecture.md
├── 11-security-design.md
├── 12-testing-strategy.md
├── 13-ci-cd.md
├── 14-observability.md
├── 15-deployment-architecture.md
├── 16-cost-analysis.md
├── 17-roadmap.md
├── 18-team-responsibilities.md
├── 19-github-repository-structure.md
├── 20-readme-specification.md
├── 21-adrs.md
├── 22-traceability.md
├── 23-interview-preparation.md
├── 24-viva-project-defense.md
└── 25-project-score.md
```

The HLD and LLD documents are already completed and should be reused rather than recreated.

---

# 5. Kernel Directory

The kernel contains the privileged bare-metal operating-system implementation.

```text
kernel/
├── Cargo.toml
├── Cargo.lock
└── src/
```

The kernel must remain independent of host user-space dependencies.

---

# 6. Kernel Source Structure

Recommended structure:

```text
kernel/src/
├── main.rs
├── boot/
├── memory/
├── interrupts/
├── input/
├── serial/
├── logger/
├── shell/
├── diagnostics/
└── panic/
```

## `main.rs`

Kernel entry point and high-level initialization sequence.

Conceptual role:

```text
Kernel Entry
    ↓
Subsystem Initialization
    ↓
Shell / Main Kernel Loop
```

---

# 7. Boot Module

```text
kernel/src/boot/
```

Responsible for:

- Boot-time initialization.
- Bootloader-related structures.
- Boot configuration.
- Initialization ordering.

The boot module should not contain unrelated shell or AI functionality.

---

# 8. Memory Module

```text
kernel/src/memory/
```

Responsible for:

- Boot memory information.
- Paging.
- Page-table management.
- Frame allocation.
- Heap initialization.
- Allocator implementation.

Potential future structure:

```text
memory/
├── mod.rs
├── paging.rs
├── frame_allocator.rs
├── heap.rs
└── allocator.rs
```

---

# 9. Interrupts Module

```text
kernel/src/interrupts/
```

Responsible for:

- Interrupt Descriptor Table.
- CPU exception handlers.
- Hardware interrupt handlers.
- Keyboard interrupt handling.
- Timer-related interrupt handling where required.

Potential structure:

```text
interrupts/
├── mod.rs
├── exceptions.rs
├── keyboard.rs
└── timer.rs
```

The exact files should be created only when the corresponding functionality is implemented.

---

# 10. Input Module

```text
kernel/src/input/
```

Responsible for:

- Keyboard event handling.
- Key decoding.
- Input buffering.
- Passing characters/events to the shell.

This separates device-specific input processing from command parsing.

---

# 11. Serial Module

```text
kernel/src/serial/
```

Responsible for:

- UART initialization.
- Serial output.
- Serial input where required.
- Kernel-to-host communication support.

This module forms one part of the controlled communication boundary.

---

# 12. Logger Module

```text
kernel/src/logger/
```

Responsible for:

- Log levels.
- Log formatting.
- Log emission.
- Logger synchronization.
- Integration with serial output.

Example levels:

```text
INFO
WARN
ERROR
PANIC
```

---

# 13. Shell Module

```text
kernel/src/shell/
```

Responsible for:

- Command prompt.
- Input buffer.
- Parser.
- Command registry.
- Command validation.
- Command dispatch.
- Command history.
- Auto-completion.

Potential structure:

```text
shell/
├── mod.rs
├── parser.rs
├── commands.rs
├── registry.rs
├── history.rs
└── completion.rs
```

---

# 14. Diagnostics Module

```text
kernel/src/diagnostics/
```

Responsible for:

- Diagnostic record creation.
- Diagnostic severity.
- Diagnostic identifiers.
- Diagnostic context.
- Protocol-ready serialization where applicable.

Conceptual structure:

```text
diagnostics/
├── mod.rs
├── record.rs
├── severity.rs
└── protocol.rs
```

---

# 15. Panic Module

```text
kernel/src/panic/
```

Responsible for:

- Panic handling.
- Panic diagnostics.
- Controlled kernel halt.
- Panic-related logging.

The panic implementation should remain lightweight because severe kernel failures may occur when normal subsystems are already unavailable.

---

# 16. AI Bridge Directory

The AI Bridge is a host-side application and should remain separate from the kernel.

```text
ai-bridge/
├── Cargo.toml
├── Cargo.lock
└── src/
```

The bridge has access to normal host operating-system facilities, including:

- Networking.
- HTTPS.
- Environment variables.
- Filesystem access where explicitly required.
- External AI APIs.

---

# 17. AI Bridge Source Structure

Recommended:

```text
ai-bridge/src/
├── main.rs
├── protocol/
├── diagnostics/
├── context/
├── prompt/
├── provider/
├── validation/
├── safety/
├── config/
└── telemetry/
```

---

# 18. Protocol Module

```text
ai-bridge/src/protocol/
```

Responsible for:

- Reading communication messages.
- Message parsing.
- Message framing.
- Message serialization.
- Protocol version checking.
- Message validation.

---

# 19. Diagnostics Module

```text
ai-bridge/src/diagnostics/
```

Responsible for:

- Receiving kernel diagnostic messages.
- Converting them into internal diagnostic models.
- Selecting relevant information.
- Preparing diagnostic context.

---

# 20. Context Module

```text
ai-bridge/src/context/
```

Responsible for constructing the context sent to the AI service.

Possible context:

```text
System Information
+
Diagnostic
+
Relevant Logs
+
User Query
```

Only necessary information should be included.

---

# 21. Prompt Module

```text
ai-bridge/src/prompt/
```

Responsible for:

- System instructions.
- Diagnostic prompts.
- User-request formatting.
- Context formatting.
- Prompt versioning.

Provider-specific prompt logic should remain isolated where practical.

---

# 22. Provider Module

```text
ai-bridge/src/provider/
```

Responsible for external AI service integration.

Potential structure:

```text
provider/
├── mod.rs
├── trait.rs
├── provider_a.rs
└── provider_b.rs
```

The implementation should use an abstraction so the AI provider can be replaced without changing the kernel communication protocol.

---

# 23. Validation Module

```text
ai-bridge/src/validation/
```

Responsible for:

- Request validation.
- Response schema validation.
- Field validation.
- Size limits.
- Protocol validation.

Malformed data must be rejected.

---

# 24. Safety Module

```text
ai-bridge/src/safety/
```

Responsible for:

- AI-output safety checks.
- Command-suggestion checks.
- Risk classification.
- Confirmation metadata.
- Prompt-injection defense policies.

The safety module must not grant the AI execution authority.

---

# 25. Configuration Module

```text
ai-bridge/src/config/
```

Responsible for:

- Provider configuration.
- Endpoint configuration.
- Model configuration.
- Timeout configuration.
- Environment loading.

Sensitive values must remain outside source control.

---

# 26. Telemetry Module

```text
ai-bridge/src/telemetry/
```

Responsible for:

- Request IDs.
- Processing duration.
- Provider status.
- Validation status.
- Error codes.
- Operational metrics.

Sensitive data should not be logged unnecessarily.

---

# 27. Tests Directory

Testing should be organized by responsibility.

```text
tests/
├── kernel/
├── shell/
├── protocol/
├── ai/
├── security/
└── integration/
```

## Kernel Tests

Test:

- Boot-related behavior where practical.
- Memory logic.
- Interrupt logic.
- Diagnostic generation.
- Panic paths.

## Shell Tests

Test:

- Parser.
- Command validation.
- Arguments.
- Unknown commands.
- Suggested-command handling.

## Protocol Tests

Test:

- Serialization.
- Deserialization.
- Message validation.
- Version handling.
- Invalid messages.

## AI Tests

Test:

- Prompt construction.
- Response parsing.
- Response validation.
- Safety handling.
- Provider errors.

## Security Tests

Test:

- Prompt injection.
- Malformed input.
- Privileged command suggestions.
- Secret exposure.
- Bypass attempts.

## Integration Tests

Test:

```text
Kernel
 ↓
Communication
 ↓
AI Bridge
 ↓
AI Service
 ↓
Response
 ↓
Psydian
```

---

# 28. Scripts Directory

The `scripts/` directory contains repeatable development commands.

```text
scripts/
├── build.sh
├── run-qemu.sh
├── test.sh
└── clean.sh
```

## `build.sh`

Responsible for:

- Building the project.
- Generating the bootable image.
- Reporting build failures.

## `run-qemu.sh`

Responsible for:

- Locating the bootable image.
- Starting QEMU.
- Configuring serial output.
- Applying the documented runtime configuration.

## `test.sh`

Responsible for:

- Running project tests.
- Running relevant checks.
- Returning a non-zero exit status when required tests fail.

## `clean.sh`

Responsible for cleaning generated build artifacts when necessary.

---

# 29. GitHub Actions

The CI workflows should live under:

```text
.github/workflows/
```

Recommended files:

```text
.github/workflows/
├── ci.yml
├── security.yml
└── release.yml
```

These correspond to the CI/CD design.

---

# 30. Assets Directory

The `assets/` directory may contain project visuals that are useful for documentation.

```text
assets/
└── diagrams/
```

Possible files:

- Architecture diagrams.
- Sequence diagrams.
- System flow diagrams.
- Project screenshots.
- Presentation graphics.

Generated build artifacts should not be mixed with documentation assets.

---

# 31. Artifacts Directory

The `artifacts/` directory may be used for locally generated demonstration artifacts when needed.

Examples:

```text
artifacts/
├── boot-image/
├── logs/
└── reports/
```

Large/generated artifacts should generally remain outside Git history unless there is a clear reason to version them.

The CI system may retain them as build artifacts instead.

---

# 32. Environment and Secrets

Local configuration may use:

```text
.env
```

or another host-side configuration mechanism.

Example variables:

```text
AI_PROVIDER=
AI_MODEL=
AI_ENDPOINT=
AI_API_KEY=
AI_TIMEOUT=
```

The `.env` file must be ignored by Git.

Example `.gitignore` entries:

```text
.env
.env.*
target/
*.log
```

Public example configuration should use a template:

```text
.env.example
```

without real credentials.

---

# 33. README Structure

The repository `README.md` should contain:

```text
# Psydian

## Overview
## Features
## Architecture
## Repository Structure
## Requirements
## Setup
## Build
## Run in QEMU
## Run AI Bridge
## Testing
## Security
## Project Status
## Roadmap
## Team
## License
```

The README should provide enough information for a new contributor to get the project running.

---

# 34. CONTRIBUTING.md

The repository may include:

```text
CONTRIBUTING.md
```

It should document:

- Branching strategy.
- Commit conventions.
- Pull-request expectations.
- Testing requirements.
- Code-formatting expectations.
- Security rules.
- How to report issues.

---

# 35. LICENSE

A project license may be included according to the team's chosen licensing requirements.

The selected license should be documented clearly.

---

# 36. Git Branching

Recommended strategy:

```text
main
develop
feature/*
```

### `main`

Stable and demonstration-ready code.

### `develop`

Optional integration branch.

### `feature/*`

Individual feature development.

Examples:

```text
feature/kernel-memory
feature/interrupts
feature/keyboard
feature/shell
feature/diagnostics
feature/ai-bridge
feature/voice
```

---

# 37. Commit Convention

Commits should describe what changed.

Examples:

```text
feat(kernel): add serial initialization
feat(shell): add command parser
feat(ai): add diagnostic bridge
fix(memory): handle allocation failure
test(protocol): add malformed message cases
docs(trd): update kernel requirements
refactor(shell): separate command registry
```

Avoid vague messages such as:

```text
update
fix
changes
final
new code
```

---

# 38. Pull Request Requirements

Every pull request should contain:

```text
Problem
Solution
Testing
Risks
Screenshots / Evidence where applicable
```

Example:

```markdown
## Problem

Kernel diagnostic messages were not identifying their source subsystem.

## Solution

Added subsystem metadata to diagnostic records.

## Testing

- Kernel build
- Protocol tests
- QEMU smoke test

## Risks

Changed diagnostic protocol structure.

## Evidence

Attached QEMU serial output.
```

---

# 39. Repository Security Rules

Never commit:

```text
API Keys
Tokens
Passwords
Private certificates
Personal secrets
```

Never intentionally commit:

```text
.env
target/
large temporary build directories
provider credentials
```

Use:

```text
.env.example
```

for documenting configuration structure.

---

# 40. Dependency Management

Each Rust component should explicitly declare its dependencies.

Dependencies should be:

- Necessary.
- Reviewed.
- Version controlled through lockfiles where appropriate.
- Updated deliberately.

Avoid adding a dependency merely because it provides a small convenience function that can easily be implemented locally.

---

# 41. Documentation-Code Synchronization

The documentation should reflect the implemented system.

When architecture changes:

```text
Implementation Change
      ↓
Review HLD/LLD impact
      ↓
Update Relevant Documentation
      ↓
Commit Documentation + Code
```

The team should avoid allowing the documentation to describe features that do not actually exist.

---

# 42. Project Status Tracking

The repository should clearly distinguish between:

```text
Implemented
In Progress
Planned
Deferred
```

Example:

```text
Boot                 ✅ Implemented
Serial               ✅ Implemented
Keyboard             🚧 In Progress
Shell                📋 Planned
AI Bridge            📋 Planned
Voice                ⏸ Deferred
```

The final project status should be synchronized with the 8-week roadmap.

---

# 43. Example Contributor Workflow

```text
Clone Repository
      ↓
Install Toolchain
      ↓
Build Project
      ↓
Run QEMU
      ↓
Create feature branch
      ↓
Implement Feature
      ↓
Run Tests
      ↓
Run QEMU Validation
      ↓
Commit
      ↓
Pull Request
      ↓
CI
      ↓
Review
      ↓
Merge
```

---

# 44. Repository Ownership

Primary source-code ownership:

```text
Kernel / Systems
→ Anurag Singh

AI Bridge / GenAI
→ Aditya Chauhan
```

Shared ownership:

```text
Documentation
Testing
Security
CI/CD
Deployment
Integration
Presentation
```

Both contributors should understand the complete system and not create isolated components that only one person can explain.

---

# 45. Repository Definition of Done

A repository change is considered complete when:

- Code is implemented.
- Relevant tests pass.
- QEMU behavior is verified where applicable.
- Documentation is updated.
- Security implications are reviewed.
- Integration impact is checked.
- The change is committed with a meaningful message.
- Required CI checks pass.
- The change is reviewable by the other team member.

---

# 46. Recommended Current Structure

Because Psydian is currently being built incrementally, the actual repository can start smaller:

```text
PsyDian_OS/
├── Docs/
│   ├── HLD.md
│   ├── LLD.md
│   ├── OJT-PRD-SEM-3(8-week).pdf
│   └── Complete-Documentation.md
│
├── kernel/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src/
│       ├── main.rs
│       └── serial/
│           └── mod.rs
│
├── .cargo/
│   └── config.toml
│
├── build.rs
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── README.md
└── .gitignore
```

As implementation progresses, additional modules should be added incrementally rather than creating empty directories for every future feature.

---

# 47. Final Repository Architecture

```text
                         PsyDian_OS
                              │
          ┌───────────────────┼───────────────────┐
          │                   │                   │
      Documentation        Kernel            AI Bridge
          │                   │                   │
          │            ┌──────┼──────┐            │
          │            │      │      │            │
        HLD/LLD     Memory  Shell  Interrupts   AI/Protocol
        BRD/PRD     Serial  Input  Diagnostics  Provider
        TRD/UX      Logger  Panic                 Validation
          │            │      │      │            Safety
          │            └──────┼──────┘            │
          │                   │                   │
          └───────────────────┼───────────────────┘
                              │
                         Integration
                              │
                    ┌─────────┴─────────┐
                    │                   │
                  Tests              CI/CD
                    │                   │
                    └─────────┬─────────┘
                              │
                           QEMU Demo
```

---

# 48. Repository Principles

### Separation of Concerns

Kernel, AI Bridge, documentation, and testing should remain logically separated.

### Simplicity

The repository should not become more complex than the project requires.

### Reproducibility

A new contributor should be able to understand how to build and run Psydian.

### Security

Secrets and sensitive configuration must remain outside source control.

### Traceability

Code, documentation, tests, commits, and milestones should be connected.

### Collaboration

Both students should be able to review and understand each other's changes.

### Incremental Growth

The repository should grow as the implementation grows rather than containing unnecessary placeholder infrastructure.

