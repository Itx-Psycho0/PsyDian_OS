# README Specification

## 1. Purpose

The root `README.md` is the primary entry point for the Psydian repository.

It should allow a new contributor, mentor, evaluator, or reviewer to understand:

- What Psydian is.
- Why the project exists.
- What has been implemented.
- How the system is architected.
- How to prepare the development environment.
- How to build the kernel.
- How to run Psydian in QEMU.
- How to run the host-side AI Bridge.
- How to test the system.
- What is currently in scope.
- What is deferred or optional.
- Where the detailed documentation is located.

The README should remain synchronized with the actual implementation.

---

# 2. README Goals

The README should be:

- Easy to scan.
- Technically accurate.
- Reproducible.
- Beginner-friendly.
- Useful for contributors.
- Useful for mentors and evaluators.
- Explicit about MVP limitations.

It should not attempt to replace the full HLD, LLD, TRD, security, testing, or GenAI documentation.

---

# 3. Recommended README Structure

```text
README.md
│
├── Project Title
├── Project Overview
├── Problem / Motivation
├── Goals
├── Key Features
├── Current Status
├── Architecture
├── Repository Structure
├── Prerequisites
├── Setup
├── Build
├── Run in QEMU
├── AI Bridge Setup
├── Testing
├── Security
├── Documentation
├── Roadmap
├── Team
├── Limitations
├── Contributing
└── License
```

---

# 4. Project Title

The README should begin with:

```markdown
# Psydian

AI-Assisted Operating System
```

A short subtitle can explain the project:

```text
A Rust-based x86_64 bare-metal operating-system prototype
with an external AI-assisted diagnostic layer.
```

---

# 5. Project Overview

The overview should explain Psydian in a few paragraphs.

Suggested content:

```markdown
## Overview

Psydian is an experimental x86_64 operating-system project written primarily in Rust.

The project focuses on building a bootable bare-metal kernel with a command-line shell, memory and interrupt foundations, structured diagnostics, and an AI-assisted troubleshooting layer.

Psydian uses an external host-side AI Bridge to connect kernel diagnostics with an AI service. The AI Bridge keeps networking and external API communication outside the privileged kernel.

The project is designed as an 8-week academic MVP and prioritizes a small, demonstrable, technically understandable operating-system core rather than attempting to implement every conventional operating-system subsystem.
```

---

# 6. Problem / Motivation

The README should explain why the project exists.

Suggested framing:

```markdown
## Motivation

Low-level operating-system failures are often difficult to understand from raw kernel logs and diagnostic information.

Psydian explores whether an AI-assisted diagnostic layer can make selected system failures easier to understand without giving the AI direct control over the privileged operating-system kernel.

The project therefore combines systems programming, Rust, bare-metal development, QEMU, structured diagnostics, and Generative AI.
```

---

# 7. Project Goals

```markdown
## Goals

### Core Goals

- Build a bootable x86_64 Rust kernel.
- Run the kernel in QEMU.
- Implement basic kernel initialization.
- Implement memory-management foundations.
- Implement interrupt and exception handling.
- Implement keyboard input.
- Implement an interactive shell.
- Implement structured kernel logging.
- Implement controlled panic/diagnostic handling.
- Implement a host-side AI Bridge.
- Integrate an external AI service.
- Provide AI-assisted diagnostic explanations.
- Validate AI-generated output before it can influence command execution.

### Engineering Goals

- Maintain modular architecture.
- Keep trust boundaries explicit.
- Use reproducible development tooling.
- Test important success and failure paths.
- Keep documentation synchronized with implementation.
```

---

# 8. Key Features

The README should list only features that are implemented or clearly marked as planned.

Example:

```markdown
## Features

### Implemented / Core

- Bootable x86_64 kernel.
- QEMU-based execution.
- Serial output.
- Kernel logging foundation.
- Bare-metal `no_std` environment.

### In Progress / Planned

- Memory-management subsystem.
- Interrupt and exception handling.
- Keyboard input.
- Interactive shell.
- Structured diagnostics.
- Host-side AI Bridge.
- AI-assisted diagnostics.

### Optional / Future

- Voice interaction.
- RAG over Psydian documentation.
- Multiple AI providers.
- Local AI models.
- UEFI support.
- Advanced shell features.
```

The actual status should be updated as development progresses.

---

# 9. Architecture

The README should provide a compact architecture overview.

```text
User
  ↓
Psydian Shell
  ↓
Psydian Kernel
  ├── Memory
  ├── Interrupts / Exceptions
  ├── Input
  ├── Serial
  ├── Logger
  └── Diagnostics
          ↓
   Serial / Diagnostic Protocol
          ↓
      Host AI Bridge
          ↓
       HTTPS / REST
          ↓
      External AI
```

The README should link to the full HLD and LLD rather than duplicating them.

---

# 10. AI Architecture Explanation

The README should explicitly explain an important design decision:

```markdown
## How the AI Works

Psydian does not place HTTP, TLS, or an external AI client directly inside the kernel for the MVP.

Instead:

1. Psydian generates a structured request or diagnostic.
2. The data crosses a controlled local communication boundary.
3. A host-side AI Bridge receives the message.
4. The AI Bridge communicates with the external AI service over HTTPS.
5. The AI response is parsed and validated.
6. The validated result is returned to Psydian.
7. The shell displays the explanation or suggestion.

This keeps external networking outside the privileged kernel.
```

---

# 11. Why There Is No Direct Kernel Networking in the MVP

Include a concise explanation:

```markdown
## Why the Kernel Does Not Directly Call the AI API

A network connection from the bare-metal kernel would require substantially more infrastructure, including a network driver, TCP/IP stack, DNS, TLS, HTTP handling, and additional reliability/security work.

For the 8-week MVP, this would add significant complexity without improving the core demonstration.

The AI Bridge therefore performs external network communication on the host while Psydian communicates with the bridge through a controlled local channel.
```

---

# 12. Repository Structure

The README should show the actual repository structure.

Example:

```text
PsyDian_OS/
├── Docs/
├── kernel/
├── ai-bridge/
├── tests/
├── scripts/
├── assets/
├── .cargo/
├── .github/
├── build.rs
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── README.md
├── CONTRIBUTING.md
├── LICENSE
└── .gitignore
```

A link should be provided to the detailed repository-structure document.

---

# 13. Prerequisites

The README should state the required environment.

```markdown
## Prerequisites

- Linux host system.
- Rust toolchain.
- Rust nightly for kernel development.
- `rust-src`.
- `llvm-tools-preview`.
- `x86_64-unknown-none` target.
- QEMU x86_64.
- Cargo.
- Git.
- Network access for external AI integration.
```

The README should state that exact versions can be checked in the project's toolchain configuration and documentation.

---

# 14. Verify the Rust Environment

Example:

```bash
rustc --version
cargo --version
rustup show
rustup target list --installed
```

Expected target:

```text
x86_64-unknown-none
```

The README should not hard-code a compiler version unless the repository intentionally pins one.

---

# 15. Build Instructions

The build section should show the simplest supported build command.

Example:

```bash
cargo build
```

When the project uses multiple packages or a workspace, the README should show the actual commands required by the repository.

The documentation should distinguish:

```text
Host build
Kernel build
Bootable image build
```

where those are separate operations.

---

# 16. Run Psydian in QEMU

The README should show the supported way to locate the bootable image.

Example:

```bash
find target -name "psydian-bios.img"
```

Then show the project-maintained QEMU script when available:

```bash
./scripts/run-qemu.sh
```

or the exact QEMU command used by the project.

The README should avoid relying on unstable generated Cargo build directories when a stable project script can provide the same function.

---

# 17. Expected Boot Output

The README can include a simple example:

```text
INFO : Entry point at: ...
INFO : Creating GDT at ...
INFO : Map framebuffer
INFO : Create bootinfo
INFO : Jumping to kernel entry point at ...
```

followed by the project's own kernel output, for example:

```text
[INFO][BOOT] Psydian kernel booted.
[INFO][SERIAL] UART initialized.
```

The exact output should be updated to match the actual implementation.

---

# 18. AI Bridge Setup

The README should explain that the AI Bridge runs on the host.

Conceptual setup:

```text
Terminal 1
└── QEMU
    └── Psydian

Terminal 2
└── AI Bridge
    └── External AI Provider
```

Example:

```bash
cd ai-bridge
cargo run
```

The exact command should follow the actual bridge implementation.

---

# 19. AI Provider Configuration

The README should document configuration without exposing credentials.

Example:

```text
AI_PROVIDER=...
AI_MODEL=...
AI_ENDPOINT=...
AI_API_KEY=...
AI_TIMEOUT=...
```

The real secret should never appear in the README.

Use:

```text
.env.example
```

to document the expected variable names.

---

# 20. Testing

The README should provide the common test commands.

Examples:

```bash
cargo fmt --all -- --check
cargo test
cargo clippy --workspace
```

Kernel-specific tests should be documented separately where they require QEMU or custom test infrastructure.

The README should distinguish:

```text
Unit Tests
Kernel/QEMU Tests
Protocol Tests
AI Bridge Tests
Security Tests
Integration Tests
```

---

# 21. AI Testing

The README should explain that live external AI calls do not need to be required for every CI run.

Recommended:

```text
Normal Tests
    ↓
Mock / Fixture AI Provider
```

Optional:

```text
Integration Test
    ↓
Real AI Provider
```

This keeps normal development/testing deterministic and avoids unnecessary API cost.

---

# 22. Security

The README should contain a short security section.

Suggested content:

```markdown
## Security

Psydian treats external AI output as untrusted data.

AI-generated command suggestions must pass through the same shell parsing, validation, and authorization flow as normal commands.

API credentials are kept on the host-side AI Bridge and must never be embedded in the kernel or committed to Git.

Core kernel operation does not depend on external AI availability.
```

A link should point to the complete Security Design document.

---

# 23. Data and Privacy

The README should mention that diagnostic information may cross the external AI boundary.

Suggested content:

```markdown
## Data Handling

Only information required for the requested diagnostic analysis should be sent to the external AI service.

Sensitive information should not be logged unnecessarily.

API keys and other credentials must remain outside source control.

The MVP should use controlled/de-identified test data where applicable.
```

---

# 24. Documentation Links

The README should link to the detailed project documents.

Example:

```markdown
## Documentation

- [BRD](Docs/02-brd.md)
- [PRD](Docs/03-prd.md)
- [UX Requirements](Docs/04-ux-requirements.md)
- [TRD](Docs/05-trd.md)
- [HLD](Docs/06-hld.md)
- [Data Design](Docs/07-database-data-design.md)
- [API Specification](Docs/08-api-specification.md)
- [LLD](Docs/09-lld.md)
- [GenAI Architecture](Docs/10-genai-architecture.md)
- [Security Design](Docs/11-security-design.md)
- [Testing Strategy](Docs/12-testing-strategy.md)
- [CI/CD](Docs/13-ci-cd.md)
- [Observability](Docs/14-observability.md)
- [Deployment Architecture](Docs/15-deployment-architecture.md)
- [Cost Analysis](Docs/16-cost-analysis.md)
- [Roadmap](Docs/17-roadmap.md)
- [Team Responsibilities](Docs/18-team-responsibilities.md)
```

The exact filenames should match the repository.

---

# 25. Current Status

The README should contain an explicitly maintained status section.

Example:

```markdown
## Current Status

| Component | Status |
|---|---|
| Repository | Complete |
| Documentation | In progress / baseline complete |
| Kernel boot | Complete |
| Serial output | Complete |
| Memory management | In progress |
| Interrupts | Planned / In progress |
| Keyboard | Planned / In progress |
| Shell | Planned / In progress |
| Diagnostics | Planned / In progress |
| AI Bridge | Planned |
| External AI integration | Planned |
| Voice | Optional / Deferred |
```

The table must reflect the actual project state.

---

# 26. Roadmap

The README should provide a compact version of the 8-week plan.

```text
Week 1 → Requirements + Environment
Week 2 → Boot + Kernel Foundation
Week 3 → Memory + Interrupts + Exceptions
Week 4 → Keyboard + Shell
Week 5 → Diagnostics + Reliability
Week 6 → AI Bridge
Week 7 → GenAI + Safety
Week 8 → Integration + Testing + Demo
```

The full roadmap should remain in the dedicated roadmap document.

---

# 27. Scope and Limitations

The README should explicitly state what Psydian does not attempt to implement in the MVP.

```markdown
## MVP Limitations

The MVP does not aim to provide:

- Full general-purpose networking inside the kernel.
- A complete production filesystem.
- A full desktop GUI.
- Full multi-user support.
- Production-grade hardware support.
- Autonomous AI control of the kernel.
- Enterprise-scale observability infrastructure.
- Production lending/decision infrastructure.
```

The exact list should match the actual Psydian PRD.

---

# 28. Optional Features

Optional features should be clearly separated from MVP requirements.

Possible future work:

```text
Voice interaction
RAG over Psydian documentation
Multiple AI providers
Local AI models
UEFI support
Advanced shell features
Persistent diagnostic history
Advanced networking
Filesystem
```

Optional features should not be presented as completed functionality.

---

# 29. Contributing

The README should direct contributors to:

```text
CONTRIBUTING.md
```

The contribution process should cover:

- Branching.
- Commits.
- Pull requests.
- Testing.
- Code review.
- Security.

Example:

```text
feature/*
    ↓
Commit
    ↓
Pull Request
    ↓
CI
    ↓
Review
    ↓
main
```

---

# 30. Team

The README should identify the project team.

Example:

```markdown
## Team

| Member | Primary Area |
|---|---|
| Anurag Singh | Kernel / Systems |
| Aditya Chauhan | AI / AI Bridge |
```

Shared areas include:

```text
Architecture
Testing
Security
CI/CD
Deployment
Documentation
Presentation
```

---

# 31. Mentor / Academic Context

Because this is an academic project, the README may include:

```text
Project Type: Academic / OJT MVP
Duration: 8 weeks
Primary Platform: x86_64 + QEMU
Primary Language: Rust
```

Any institution, course, or mentor-specific information should be included only if required by the project's official documentation.

---

# 32. License

The README should include the selected license.

Example:

```markdown
## License

This project is licensed under the terms specified in [LICENSE](LICENSE).
```

Do not claim a license before the repository actually contains the corresponding license file.

---

# 33. README Quality Checklist

Before considering the README complete, verify:

- [ ] Project name is correct: Psydian.
- [ ] Description matches the actual implementation.
- [ ] MVP scope is clear.
- [ ] Architecture is understandable.
- [ ] Repository structure is current.
- [ ] Prerequisites are correct.
- [ ] Build commands work.
- [ ] QEMU run instructions work.
- [ ] AI Bridge instructions work when implemented.
- [ ] Testing commands are correct.
- [ ] Security rules are visible.
- [ ] Secrets are not included.
- [ ] Documentation links work.
- [ ] Current status is accurate.
- [ ] Roadmap matches the 8-week schedule.
- [ ] Deferred features are labeled correctly.
- [ ] Team information is correct.

---

# 34. Recommended Final README Layout

The actual root README should remain concise.

```text
# Psydian

One-paragraph project overview.

## Features

Core features.

## Architecture

Small architecture diagram.

## Repository Structure

Small directory tree.

## Prerequisites

Required tools.

## Setup

Installation/setup commands.

## Build

Build command.

## Run

QEMU execution command.

## AI Bridge

Host-side AI setup.

## Testing

Main test commands.

## Security

Short security rules.

## Documentation

Links to detailed documents.

## Roadmap

8-week summary.

## Current Status

Current implementation state.

## Team

Team members and responsibilities.

## Limitations

MVP boundaries.

## License

License information.
```

---

# 35. README Principle

The root README should answer five questions quickly:

```text
What is Psydian?
        ↓
Why does it exist?
        ↓
How is it structured?
        ↓
How do I run it?
        ↓
Where can I learn more?
```

Detailed technical explanations should remain in the project's dedicated documentation.

---

# 36. Final README Requirement

The root `README.md` is considered complete when a new developer can clone the repository, follow the documented prerequisites and setup steps, build the project, start the supported QEMU environment, understand the kernel/AI Bridge boundary, run the relevant tests, and locate the detailed architecture and security documentation without requiring undocumented manual steps.
