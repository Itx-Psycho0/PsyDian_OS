# Deployment Architecture

## 1. Purpose

The deployment architecture for Psydian defines how the kernel, QEMU environment, host-side AI Bridge, and external AI service are developed, executed, tested, and demonstrated.

Psydian is not a conventional web application. The MVP primarily runs as a bootable x86_64 operating-system image inside QEMU, while the AI Bridge runs separately as a normal host-side application.

The deployment architecture therefore separates:

- Bare-metal kernel execution.
- Virtual hardware execution.
- Host-side AI processing.
- External AI service access.

---

# 2. Deployment Architecture Overview

```text
                    DEVELOPMENT HOST
┌─────────────────────────────────────────────────────┐
│                                                     │
│  Linux Host                                          │
│                                                     │
│  ┌───────────────────────┐                          │
│  │ Psydian Build System  │                          │
│  │ Rust + Cargo          │                          │
│  └───────────┬───────────┘                          │
│              ↓                                      │
│       Bootable Image                                │
│              ↓                                      │
│  ┌───────────────────────┐                          │
│  │ QEMU                  │                          │
│  │                       │                          │
│  │ ┌─────────────────┐  │                          │
│  │ │ Psydian Kernel  │  │                          │
│  │ │ + Shell         │  │                          │
│  │ │ + Diagnostics   │  │                          │
│  │ └────────┬────────┘  │                          │
│  └──────────┼────────────┘                          │
│             │                                       │
│       Serial / Diagnostic Channel                   │
│             │                                       │
│  ┌──────────▼────────────┐                          │
│  │ Host AI Bridge        │                          │
│  │                       │                          │
│  │ Protocol Processing   │                          │
│  │ AI Client             │                          │
│  └──────────┬────────────┘                          │
│             │                                       │
└─────────────┼───────────────────────────────────────┘
              │ HTTPS / REST
              ▼
       External AI Service
```

---

# 3. Deployment Environments

The project uses the following development and validation stages:

```text
Development
    ↓
Testing
    ↓
Stable / Demo
```

A traditional production deployment is not required for the 8-week MVP.

---

# 4. Development Environment

The development environment consists of:

```text
Linux Host
   +
Rust Toolchain
   +
Cargo
   +
QEMU
   +
Git/GitHub
   +
Host AI Bridge
```

The same environment should be documented so that both team members can reproduce the project.

## Development Responsibilities

Developers may:

- Modify kernel source.
- Modify shell/source modules.
- Modify AI Bridge source.
- Build the bootable image.
- Run QEMU.
- Execute tests.
- Inspect serial logs.
- Test AI integration.

---

# 5. Testing Environment

Testing should use a clean and controlled configuration as much as practical.

The testing environment shall validate:

- Kernel compilation.
- Bootloader integration.
- QEMU boot.
- Kernel output.
- Shell behavior.
- Diagnostics.
- Communication protocol.
- AI Bridge behavior.
- AI response validation.
- Security controls.

Conceptual flow:

```text
Source
  ↓
Build
  ↓
Bootable Image
  ↓
QEMU
  ↓
Kernel Tests
  ↓
AI Bridge Tests
  ↓
Integration Tests
```

---

# 6. Stable / Demo Environment

The stable environment represents the state used for:

- Mentor evaluation.
- Project demonstration.
- Final testing.
- Screenshots/recordings.
- Presentation.

The `main` branch should contain a stable version of the system that can be rebuilt and demonstrated.

Conceptually:

```text
Validated Code
    ↓
main
    ↓
Release Build
    ↓
Psydian BIOS Image
    ↓
QEMU
    ↓
Demo
```

---

# 7. Kernel Deployment

The kernel is not deployed like a conventional Linux application.

The deployment unit is a bootable image.

```text
Rust Kernel
    ↓
Kernel Binary
    ↓
Bootloader
    ↓
Bootable BIOS Image
    ↓
QEMU
```

For the MVP, the primary artifact is:

```text
psydian-bios.img
```

This image is used to start Psydian under QEMU.

---

# 8. QEMU Deployment

QEMU provides the runtime environment for the kernel.

The bootable image is attached as a virtual disk.

Conceptually:

```text
psydian-bios.img
        ↓
QEMU Virtual Disk
        ↓
Virtual x86_64 Machine
        ↓
Bootloader
        ↓
Psydian Kernel
```

Serial output can be redirected to the host terminal for diagnostics:

```text
Psydian Kernel
      ↓
Virtual UART
      ↓
QEMU Serial
      ↓
Host Terminal
```

---

# 9. Host-side AI Bridge Deployment

The AI Bridge runs outside QEMU as a normal host-side process.

Conceptually:

```text
Host Linux
    │
    ├── QEMU
    │    └── Psydian
    │
    └── AI Bridge
         └── External AI API
```

This separation is intentional.

The AI Bridge has access to normal host operating-system networking capabilities while the kernel remains independent of the external network stack.

---

# 10. AI Bridge Configuration

The AI Bridge should receive configuration through the host environment or a protected configuration mechanism.

Possible configuration values:

```text
AI_PROVIDER
AI_MODEL
AI_ENDPOINT
AI_API_KEY
AI_TIMEOUT
PROTOCOL_VERSION
```

Secrets must not be embedded into the kernel image.

They must also not be committed to Git.

---

# 11. External AI Service Deployment

The external AI service is treated as a dependency rather than as part of Psydian itself.

The architecture is:

```text
Psydian
   ↓
AI Bridge
   ↓
HTTPS / REST
   ↓
External AI Provider
```

The exact provider and model can be selected during implementation.

Provider-specific details should remain isolated within the AI Bridge.

---

# 12. Network Boundary

The kernel does not directly access the external network for the MVP.

Instead:

```text
Kernel
   ↓
Serial / Diagnostic Channel
   ↓
Host AI Bridge
   ↓
Host Networking Stack
   ↓
HTTPS
   ↓
External AI
```

This means the MVP does not require:

- TCP/IP implementation inside the kernel.
- DNS implementation inside the kernel.
- TLS implementation inside the kernel.
- HTTP client implementation inside the kernel.

---

# 13. Deployment Security

Deployment must preserve the established trust boundaries.

```text
Trusted:
Kernel
Shell
Command Validator
Diagnostics

Controlled Boundary:
Serial / Protocol
     ↓
AI Bridge

External / Untrusted:
AI Service
```

Security requirements include:

- API credentials remain on the host.
- API credentials are never embedded in the kernel.
- AI-generated content remains untrusted.
- Kernel privileges are never directly exposed to the AI provider.
- External service failures remain isolated from kernel execution.

---

# 14. Artifact Management

Important build artifacts include:

```text
Kernel ELF/Binary
Bootable BIOS Image
Test Logs
QEMU Serial Logs
AI Bridge Build
Test Reports
```

Generated artifacts should be associated with a known Git commit or release version.

This makes demonstration and debugging easier.

---

# 15. Versioning

Project releases should be associated with Git tags where practical.

Example progression:

```text
v0.1.0
  Kernel boots

v0.2.0
  Shell functional

v0.3.0
  Diagnostics functional

v0.4.0
  AI Bridge integrated

v1.0.0
  MVP demonstration
```

The exact versioning scheme may be simplified if required by the team.

---

# 16. Rollback

The deployment process should allow returning to a previously known-good build.

For example:

```text
Current Build
     ↓
Failure
     ↓
Select Known-Good Git Commit/Tag
     ↓
Rebuild
     ↓
Generate Bootable Image
     ↓
Run QEMU
```

Because the MVP is primarily a development/demo system, rollback is based mainly on:

- Git commits.
- Git tags.
- Reproducible builds.
- Stored bootable artifacts.

---

# 17. Database Migration

A traditional database migration process is not required for the Psydian MVP because the core system does not depend on a conventional persistent database.

If future versions introduce persistent diagnostic history or user data, explicit schema versioning and migration scripts should be introduced at that time.

---

# 18. Deployment Failure Handling

## Kernel Build Failure

```text
Build Failure
    ↓
Inspect Rust/Cargo Error
    ↓
Fix Source or Dependency
    ↓
Rebuild
```

## QEMU Boot Failure

```text
Boot Failure
    ↓
Inspect Bootloader / Kernel Logs
    ↓
Validate Boot Image
    ↓
Rebuild
    ↓
Retry
```

## AI Bridge Failure

```text
Bridge Failure
    ↓
Core Psydian Continues
    ↓
AI Assistance Unavailable
```

## External AI Failure

```text
AI Provider Failure
    ↓
Bridge Reports Controlled Error
    ↓
Psydian Remains Operational
```

---

# 19. Deployment Observability

Deployment validation should monitor:

- Build result.
- Boot success.
- Kernel initialization.
- QEMU serial output.
- AI Bridge startup.
- Communication status.
- AI provider availability.
- End-to-end diagnostic response.

Example stable boot evidence:

```text
[INFO][BOOT] Psydian kernel booted.
[INFO][SERIAL] UART initialized.
[INFO][MEMORY] Memory initialized.
[INFO][SHELL] Shell initialized.
```

---

# 20. Demo Deployment Flow

Before the final demonstration:

```text
Clean Repository
      ↓
Checkout Stable Commit
      ↓
Verify Toolchain
      ↓
Build Project
      ↓
Generate Bootable Image
      ↓
Start QEMU
      ↓
Verify Kernel Boot
      ↓
Verify Shell
      ↓
Verify Diagnostics
      ↓
Start AI Bridge
      ↓
Verify AI Communication
      ↓
Run End-to-End AI Diagnostic
      ↓
Capture Evidence
```

---

# 21. Final Demonstration Architecture

The intended demonstration environment is:

```text
                 HOST LINUX
┌─────────────────────────────────────┐
│                                     │
│   ┌─────────────────────────────┐   │
│   │           QEMU              │   │
│   │                             │   │
│   │       Psydian Kernel        │   │
│   │            ↓                │   │
│   │          Shell              │   │
│   │            ↓                │   │
│   │       Diagnostics           │   │
│   └────────────┬────────────────┘   │
│                │                    │
│          Serial / Protocol          │
│                │                    │
│   ┌────────────▼────────────────┐   │
│   │         AI Bridge            │   │
│   └────────────┬────────────────┘   │
│                │                    │
└────────────────┼────────────────────┘
                 │
              HTTPS
                 │
                 ▼
          External AI Service
```

---

# 22. Physical Hardware Deployment

Physical hardware execution is outside the primary MVP deployment target.

The initial target is:

```text
x86_64
+
QEMU
```

Physical hardware can be considered after the kernel is stable.

Potential future deployment targets include:

- Real x86_64 hardware.
- UEFI-based systems.
- Dedicated test machines.

Any physical deployment should be treated as a separate validation stage because hardware behavior can differ from QEMU.

---

# 23. UEFI Deployment

The initial deployment target is BIOS/QEMU.

UEFI support is a future enhancement unless it becomes necessary for the MVP.

The future architecture may support:

```text
BIOS
    ↓
Psydian

UEFI
    ↓
Psydian
```

The two boot paths should eventually converge at the same kernel interface.

---

# 24. Containerization

The bare-metal kernel itself is not deployed as a normal container.

Containerization may, however, be useful for host-side services such as:

```text
AI Bridge
Testing Utilities
Development Services
```

Containerization is optional and should only be introduced when it improves reproducibility or deployment simplicity.

---

# 25. CI/CD Integration

The deployment process should integrate with the CI/CD pipeline:

```text
Git Push
    ↓
CI
    ↓
Test
    ↓
Build Kernel
    ↓
Build Boot Image
    ↓
Smoke Test
    ↓
Artifact
    ↓
Stable Build / Demo
```

Only validated builds should be used for final demonstrations.

---

# 26. Deployment Requirements

| ID | Requirement | Priority |
|---|---|---|
| DEP-001 | Psydian shall be buildable from a clean repository using the documented toolchain. | Must |
| DEP-002 | The build shall generate a bootable x86_64 BIOS image. | Must |
| DEP-003 | The image shall run in QEMU. | Must |
| DEP-004 | Kernel serial output shall be accessible from the host environment. | Must |
| DEP-005 | AI Bridge shall run independently from the kernel. | Must |
| DEP-006 | AI Bridge shall communicate with the external AI service through HTTPS/REST. | Must |
| DEP-007 | AI provider credentials shall remain outside the kernel and source repository. | Must |
| DEP-008 | AI service failure shall not prevent the core kernel from running. | Must |
| DEP-009 | Stable builds shall be associated with identifiable Git commits/tags. | Should |
| DEP-010 | Bootable images and useful test logs should be retained as build artifacts. | Should |
| DEP-011 | Deployment should support rollback to a known-good build. | Should |
| DEP-012 | The final demo environment shall be reproducible. | Must |

---

# 27. MVP Deployment Scope

## Required

- Linux development host.
- Rust nightly toolchain.
- x86_64 kernel build.
- Bootloader.
- Bootable BIOS image.
- QEMU execution.
- Serial output.
- Host-side AI Bridge.
- External AI API access.
- Environment-based secret management.
- Stable demo build.
- Build/test documentation.

## Optional

- Automated QEMU deployment through CI.
- UEFI deployment.
- Physical-hardware deployment.
- AI Bridge containerization.
- Remote monitoring.
- Persistent deployment telemetry.
- Automated release publishing.
- Artifact signing.

Optional deployment infrastructure must not delay the core 8-week MVP.

---

# 28. Final Deployment Flow

```text
Developer
    ↓
Git Repository
    ↓
CI / Local Build
    ↓
Kernel Compilation
    ↓
Bootloader Image Generation
    ↓
Psydian BIOS Image
    ↓
QEMU
    ↓
Psydian Kernel
    ↓
Shell / Diagnostics
    ↓
Serial Communication
    ↓
Host AI Bridge
    ↓
HTTPS / REST
    ↓
External AI Service
```

---

# 29. Deployment Principles

### Reproducibility

The project should be rebuildable from the documented source and toolchain.

### Isolation

The kernel, AI Bridge, and external AI service should remain separate execution environments.

### Security

Secrets must remain outside the kernel and source repository.

### Fault Isolation

External AI failures must not cause kernel failures.

### Demonstrability

The stable build should be easy to start and verify in QEMU.

### Minimal Complexity

Deployment infrastructure should remain proportional to the scope of an 8-week academic MVP.
