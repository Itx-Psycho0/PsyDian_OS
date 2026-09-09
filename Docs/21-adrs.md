# Architecture Decision Records (ADRs)

## 1. Purpose

Architecture Decision Records document important technical decisions made during the development of Psydian.

An ADR records:

- The decision.
- The problem that required a decision.
- The options considered.
- The selected option.
- The reasoning behind the selection.
- The consequences.
- The status of the decision.

ADRs prevent important architectural decisions from becoming undocumented assumptions.

---

# 2. ADR Format

Each ADR should follow this structure:

```text
ADR-XXX: Decision Title

Status:
Date:
Decision:

Context:

Options Considered:

Decision Rationale:

Consequences:

Implementation Notes:

Related Requirements:
Related Documents:
```

Possible statuses:

```text
PROPOSED
ACCEPTED
REJECTED
SUPERSEDED
DEPRECATED
```

---

# 3. ADR Index

| ADR | Decision | Status |
|---|---|---|
| ADR-001 | Use Rust for the kernel | Accepted |
| ADR-002 | Use `no_std` for the kernel | Accepted |
| ADR-003 | Target x86_64 for the MVP | Accepted |
| ADR-004 | Use QEMU as the primary development environment | Accepted |
| ADR-005 | Use the Rust bootloader ecosystem | Accepted |
| ADR-006 | Use a host-side AI Bridge instead of direct kernel networking | Accepted |
| ADR-007 | Use serial communication as the initial kernel/host boundary | Accepted |
| ADR-008 | Treat AI output as untrusted data | Accepted |
| ADR-009 | Keep core kernel operation independent of AI availability | Accepted |
| ADR-010 | Keep voice integration optional for the MVP | Accepted |
| ADR-011 | Avoid a conventional database in the MVP | Accepted |
| ADR-012 | Use structured diagnostic messages | Accepted |

---

# ADR-001: Use Rust for the Kernel

## Status

**ACCEPTED**

## Context

Psydian requires low-level control over CPU execution, memory management, interrupts, device interaction, and kernel initialization.

The project also needs a language suitable for systems programming and bare-metal development.

## Options Considered

### C

Advantages:

- Widely used for operating-system development.
- Large systems-programming ecosystem.
- Direct low-level control.

Disadvantages:

- Memory-safety problems are easier to introduce.
- Modern abstractions require more manual discipline.

### C++

Advantages:

- Strong systems-programming capabilities.
- Object-oriented and generic programming support.
- Large ecosystem.

Disadvantages:

- More language complexity.
- Bare-metal runtime/library considerations.
- Manual memory-safety concerns remain.

### Rust

Advantages:

- Systems-programming capabilities.
- Compile-time memory-safety guarantees for safe Rust.
- Strong type system.
- Support for `no_std`.
- Good ecosystem for bare-metal development.

## Decision

Use **Rust** as the primary implementation language for the Psydian kernel.

## Decision Rationale

Rust provides the low-level control required by the operating system while reducing a class of memory-safety errors through compile-time checks.

It also aligns with the project's goal of learning modern systems programming.

## Consequences

### Positive

- Strong type safety.
- Memory-safety guarantees in safe code.
- Suitable bare-metal ecosystem.
- Good maintainability.

### Negative

- More complex learning curve.
- Some kernel functionality requires `unsafe` Rust.
- Some tools and workflows may require nightly Rust.

## Related Requirements

```text
TR-001
TR-002
TR-003
```

---

# ADR-002: Use `no_std` for the Kernel

## Status

**ACCEPTED**

## Context

The Psydian kernel runs without an underlying operating system.

The standard Rust library depends on operating-system services that are not available to a freestanding kernel.

## Decision

Use:

```rust
#![no_std]
```

for the kernel.

## Decision Rationale

The kernel must operate directly in a bare-metal environment and cannot assume access to normal operating-system services.

## Consequences

### Positive

- Suitable for freestanding execution.
- Explicit control over available runtime facilities.
- Appropriate for kernel development.

### Negative

- Standard library facilities are unavailable.
- Common abstractions may need to be implemented or replaced.
- Memory allocation and I/O require explicit kernel support.

---

# ADR-003: Target x86_64 for the MVP

## Status

**ACCEPTED**

## Context

The project requires a practical hardware architecture for implementing and testing:

- CPU exceptions.
- Interrupts.
- Paging.
- Memory management.
- Keyboard input.
- Boot procedures.

## Options Considered

### x86_64

- Widely used.
- Well supported by QEMU.
- Appropriate for desktop/server systems.
- Strong documentation and tooling.

### ARM64

- Modern architecture.
- Widely used in embedded/mobile systems.

Disadvantages for this MVP include additional platform-specific setup and a less direct match with the current development environment.

### RISC-V

- Open instruction-set architecture.
- Educational value.

Disadvantages include a different platform ecosystem and additional scope for an 8-week MVP.

## Decision

Target:

```text
x86_64-unknown-none
```

for the MVP.

## Consequences

The kernel implementation and hardware abstractions will initially be x86_64-specific.

Future architecture support should be isolated behind appropriate abstractions.

---

# ADR-004: Use QEMU as the Primary Development Environment

## Status

**ACCEPTED**

## Context

Running an experimental kernel directly on physical hardware can be slow and risky during early development.

The project requires repeatable kernel execution and controlled failure testing.

## Decision

Use **QEMU** as the primary runtime and testing environment.

## Decision Rationale

QEMU provides an isolated virtual machine that can repeatedly boot the kernel without rebooting the host system.

It also provides virtual hardware and serial-device support suitable for the MVP.

## Consequences

### Positive

- Safe kernel experimentation.
- Repeatable testing.
- Easy reboot cycles.
- Debugging without risking the host OS.
- Suitable for automated smoke tests.

### Negative

- QEMU behavior is not identical to every physical machine.
- Hardware-specific bugs may appear later on real hardware.

---

# ADR-005: Use the Rust Bootloader Ecosystem

## Status

**ACCEPTED**

## Context

Implementing the entire PC boot process from firmware through kernel entry would consume significant project time.

The project's main objective is the kernel and AI-assisted diagnostic architecture.

## Decision

Use the Rust bootloader ecosystem to generate/load the bootable Psydian kernel image.

## Decision Rationale

This allows the team to focus on kernel functionality while still producing a real bootable system.

## Consequences

### Positive

- Faster path to a bootable kernel.
- Reduced boot-protocol implementation scope.
- Better alignment with the 8-week schedule.

### Negative

- Some boot behavior is controlled by the selected bootloader.
- The project depends on bootloader compatibility.

---

# ADR-006: Use a Host-side AI Bridge

## Status

**ACCEPTED**

## Context

The Psydian kernel does not initially contain:

- TCP/IP.
- DNS.
- TLS.
- HTTP.
- An external network stack.

Connecting directly from the kernel to an external AI service would significantly increase project complexity.

## Options Considered

### Option A — Direct Kernel Networking

```text
Kernel
 ↓
Network Driver
 ↓
TCP/IP
 ↓
TLS
 ↓
HTTP
 ↓
AI API
```

### Option B — Host-side AI Bridge

```text
Kernel
 ↓
Controlled Communication
 ↓
AI Bridge
 ↓
HTTPS
 ↓
AI Service
```

## Decision

Use **Option B: a host-side AI Bridge**.

## Decision Rationale

The bridge preserves the current kernel scope while still enabling external AI integration.

It also creates a useful security and abstraction boundary.

## Consequences

### Positive

- No full networking stack required in the kernel MVP.
- Simpler kernel.
- Easier API-provider replacement.
- Better fault isolation.
- Easier AI testing.

### Negative

- Additional host-side component.
- Communication protocol must be designed and maintained.
- AI assistance depends on the host bridge.

---

# ADR-007: Use Serial Communication as the Initial Kernel/Host Boundary

## Status

**ACCEPTED**

## Context

The kernel requires a simple communication mechanism that works before a full network stack or advanced device layer exists.

## Options Considered

### Serial / UART

Simple, lightweight, and suitable for early kernel development.

### Shared Memory

Potentially fast but introduces more synchronization and virtualization complexity.

### Native Networking

Too large in scope for the MVP.

## Decision

Use **UART/serial communication** as the primary initial communication and diagnostic channel.

## Decision Rationale

Serial output is already useful for kernel logging and debugging, and the same mechanism can support the early host communication boundary.

## Consequences

### Positive

- Simple implementation.
- Useful for debugging.
- Works naturally with QEMU.
- Low protocol complexity.

### Negative

- Limited bandwidth.
- Requires careful message framing.
- Not intended as a final high-performance communication architecture.

---

# ADR-008: Treat AI Output as Untrusted Data

## Status

**ACCEPTED**

## Context

AI-generated output may contain:

- Incorrect explanations.
- Hallucinated facts.
- Unsafe commands.
- Prompt-injection responses.
- Unsupported recommendations.

The AI must therefore not be considered an authority over the operating system.

## Decision

Treat all AI-generated content as **untrusted data**.

## Decision Rationale

The kernel and shell remain the trusted execution components.

The AI provides:

```text
Explanation
Suggestion
Interpretation
```

but not:

```text
Authorization
Privilege
Direct execution
```

## Consequences

Any suggested command must pass through:

```text
Parser
 ↓
Validation
 ↓
Risk Classification
 ↓
User Confirmation
 ↓
Execution
```

This adds validation work but significantly improves safety.

---

# ADR-009: Keep Core Kernel Operation Independent of AI

## Status

**ACCEPTED**

## Context

External AI services can fail because of:

- Network outages.
- API failures.
- Rate limits.
- Authentication problems.
- Provider downtime.
- Timeouts.

A diagnostic assistant must not become a single point of failure for the operating system itself.

## Decision

Core Psydian kernel and shell functionality shall remain operational when the AI service is unavailable.

## Expected Behavior

```text
AI Available
    ↓
AI Assistance Works

AI Unavailable
    ↓
Controlled Error
    ↓
Core Shell Continues
```

## Consequences

### Positive

- Better reliability.
- Clear subsystem boundaries.
- Easier local testing.
- AI remains optional assistance.

### Negative

- Some advanced diagnostic functionality is unavailable during AI failure.

---

# ADR-010: Keep Voice Integration Optional for the MVP

## Status

**ACCEPTED**

## Context

Voice functionality may require:

- Speech-to-text.
- Text-to-speech.
- Audio input/output.
- Additional libraries or services.
- Additional integration and testing.

The project has only 8 weeks.

## Decision

Voice integration is an optional/stretch feature and must not block completion of the core MVP.

## Preferred Architecture

```text
Voice
 ↓
Speech-to-Text
 ↓
Existing Shell / AI Pipeline
 ↓
Response
 ↓
Text-to-Speech
```

## Decision Rationale

Reusing the existing pipeline reduces architectural duplication.

## Consequences

Voice can be added after the core kernel, shell, diagnostics, and AI Bridge are stable.

---

# ADR-011: Avoid a Conventional Database in the MVP

## Status

**ACCEPTED**

## Context

The mentor documentation template contains a database-design section, but Psydian's core functionality does not require a conventional application database.

The MVP is primarily a kernel and host-side AI system.

## Decision

Do not introduce MongoDB, PostgreSQL, or another conventional database solely to match a template.

## Data Model Instead

Psydian will use:

```text
Kernel Runtime Structures
+
Diagnostic Records
+
Communication Messages
+
Shell State
+
AI Request/Response Structures
+
Host Configuration
```

## Decision Rationale

A database would add infrastructure and development complexity without providing a necessary MVP capability.

## Future Trigger

A database may be reconsidered if the system gains:

- Persistent diagnostic history.
- Multi-user support.
- Remote monitoring.
- Long-term telemetry.
- Persistent AI interaction history.

---

# ADR-012: Use Structured Diagnostic Messages

## Status

**ACCEPTED**

## Context

Raw log text is useful for humans but difficult to process reliably as machine input.

AI-assisted diagnostics require consistent structured information.

## Decision

Represent important diagnostic events using structured messages.

Conceptual example:

```json
{
  "type": "diagnostic",
  "version": 1,
  "id": "diag-001",
  "severity": "error",
  "subsystem": "memory",
  "message": "Heap allocation failed"
}
```

## Decision Rationale

Structured diagnostics improve:

- Validation.
- AI context generation.
- Testing.
- Debugging.
- Versioning.
- Observability.

## Consequences

The project must define:

- Message types.
- Message fields.
- Versioning.
- Message framing.
- Validation rules.

---

# 4. Architecture Decisions Still Open

Some decisions should remain open until implementation evidence is available.

| Decision | Current State | Decision Point |
|---|---|---|
| Exact AI provider | Open | Before Week 7 |
| Exact AI model | Open | During AI evaluation |
| Exact kernel/host protocol encoding | Open | During Week 5–6 |
| Exact memory allocator | Open | During memory implementation |
| Shell command set | Evolving | During Week 4 |
| Voice technology | Open | Only if optional feature begins |
| RAG implementation | Deferred | Only if concrete retrieval need appears |
| Physical hardware support | Deferred | After MVP |
| UEFI priority | Deferred | After BIOS MVP is stable |

Open decisions should become ADRs when they materially affect system architecture.

---

# 5. ADR Lifecycle

The project should follow:

```text
Technical Question
      ↓
Identify Options
      ↓
Evaluate Trade-offs
      ↓
Make Decision
      ↓
Create ADR
      ↓
Implement
      ↓
Review
```

---

# 6. Superseding an ADR

When a previous decision changes, do not silently edit history.

Instead:

```text
ADR-006
Status: SUPERSEDED
        ↓
ADR-013
New Decision
```

The new ADR should reference the previous decision.

Example:

```text
Supersedes: ADR-006
```

---

# 7. ADR Quality Requirements

An ADR should:

- Describe one significant decision.
- Explain the context.
- Mention alternatives considered.
- Explain why the chosen option was selected.
- Describe consequences.
- Avoid documenting implementation details that do not influence the decision.
- Remain understandable to another developer months later.

---

# 8. Final ADR Principles

### Explicit Decisions

Important architecture choices should be recorded rather than assumed.

### Evidence-Based

Prefer decisions supported by technical constraints, prototypes, tests, and project scope.

### Reversible Where Possible

Avoid unnecessary decisions that permanently constrain future architecture.

### Scope-Aware

The best architecture for Psydian is the architecture that can realistically be implemented and validated within the 8-week MVP.

### Security-Aware

Decisions involving AI, communication, privileges, and external services must consider the trust boundary.

### Documentation-Synchronized

When an accepted ADR changes the architecture, related HLD, LLD, TRD, security, or implementation documentation should be reviewed.
