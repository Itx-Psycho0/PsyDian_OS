
# Technical Requirements Document (TRD)

## 1. Proposed Architecture

Psydian will use a modular bare-metal operating-system architecture with a clear separation between the privileged kernel environment and the host-side AI assistance layer.

### 1.1 High-Level Technical Architecture

```text
User
  ↓
Psydian Shell
  ↓
Psydian Kernel
  ├── Boot Initialization
  ├── Interrupt / Exception Handling
  ├── Memory Management
  ├── Keyboard Input
  ├── Serial Communication
  ├── Logger
  └── Panic / Diagnostics
          ↓
   Serial / Diagnostic Channel
          ↓
     Host-side AI Bridge
          ↓
       HTTPS / REST
          ↓
   External AI Service
          ↓
      AI Response
          ↓
    Response Validation
          ↓
   Serial / Diagnostic Channel
          ↓
      Psydian Shell
          ↓
          User
```

### 1.2 Execution Environments

#### Kernel Environment

The kernel is a bare-metal Rust program targeting x86_64.

Responsibilities include:

- Boot-time initialization.
- CPU exception and interrupt handling.
- Memory-management initialization.
- Keyboard/input handling.
- Serial communication.
- Kernel logging.
- Panic handling.
- Structured system diagnostics.
- Supporting the command shell.

The kernel shall not directly depend on an external network connection or AI API for its basic operation.

#### QEMU Environment

QEMU provides the virtual x86_64 machine used during development.

It provides:

- Virtual CPU.
- Virtual memory.
- Virtual devices.
- Firmware/boot environment.
- Serial-device emulation.
- Repeatable execution environment.

QEMU is the primary development and validation environment for the MVP.

#### Host-side AI Bridge Environment

The AI Bridge runs as a normal host-side application outside the Psydian kernel.

Responsibilities include:

- Receiving system requests and diagnostics.
- Parsing the communication protocol.
- Preparing AI requests.
- Communicating with an external AI service.
- Validating AI responses.
- Returning structured results to Psydian.
- Handling API failures and timeouts.

### 1.3 Communication Boundary

The communication architecture is intentionally separated:

```text
Psydian Kernel
      ↓
Serial / Diagnostic Channel
      ↓
Host AI Bridge
      ↓
HTTPS / REST API
      ↓
External AI Service
```

The kernel therefore does not require a native TCP/IP stack, DNS resolver, TLS implementation, or HTTP client for the MVP.

### 1.4 Architecture Principles

- Keep kernel functionality independent of AI availability.
- Keep external network communication outside the privileged kernel.
- Use explicit interfaces between major subsystems.
- Treat AI-generated content as untrusted data.
- Validate AI-generated suggestions before any action is considered.
- Require explicit confirmation for privileged or destructive operations.
- Keep optional voice functionality separate from core command processing.
- Keep subsystem responsibilities modular.
- Prefer implementations that are realistic within the 8-week project timeline.

---

# 2. Technology Stack

## 2.1 Core Technology Stack

| Technology | Purpose |
|---|---|
| Rust | Primary implementation language |
| `no_std` | Freestanding kernel environment |
| x86_64 | Target CPU architecture |
| Cargo | Rust build and dependency management |
| Rust nightly | Kernel/bootloader development toolchain |
| `bootloader_api` | Kernel-facing bootloader interface |
| `bootloader` | Bootable image generation |
| QEMU | x86_64 virtual machine and testing environment |
| UART / Serial | Kernel output and host communication |
| `uart_16550` | UART abstraction |
| `spin` | Lightweight synchronization for kernel shared resources |
| Git / GitHub | Version control and collaboration |
| Host Linux | Development and AI Bridge environment |
| REST / HTTPS | Host-to-external-AI communication |

## 2.2 AI Technology

The AI layer will be implemented as a host-side service rather than as part of the privileged kernel.

The AI layer will be responsible for:

- Natural-language assistance.
- System diagnostic interpretation.
- Troubleshooting explanations.
- Suggested recovery steps.
- Optional command suggestions.

The exact AI provider will be selected separately and should remain replaceable through the AI Bridge abstraction.

## 2.3 Optional Voice Technology

Voice integration is an optional feature.

The intended architecture is:

```text
Voice Input
    ↓
Speech-to-Text
    ↓
Recognized Text
    ↓
Existing Shell / AI Pipeline
    ↓
Response
    ↓
Text-to-Speech
    ↓
Voice Output
```

Voice functionality will not create an independent command-execution architecture.

---

# 3. Technology Rationale

## 3.1 Rust

Rust is selected as the primary kernel language because the project requires low-level control over CPU, memory, interrupts, and device interaction while benefiting from compile-time safety guarantees where possible.

Rust also provides an established ecosystem for bare-metal and operating-system development.

### Used for

- Kernel.
- Shell.
- Memory-management components.
- Interrupt/exception handling.
- Logging.
- Diagnostics.
- Host-side components where appropriate.

## 3.2 `no_std`

The kernel uses a freestanding Rust environment rather than the normal standard library environment.

```rust
#![no_std]
```

This is required because the Psydian kernel does not execute on top of Linux or another operating system and therefore cannot assume the operating-system services normally provided by `std`.

## 3.3 x86_64

x86_64 is selected for the MVP because:

- It is a widely used desktop/server architecture.
- It is directly supported by QEMU.
- It exposes the CPU and memory-management concepts required by the project.
- It provides a well-established environment for systems-programming learning.

The current kernel compilation target is:

```text
x86_64-unknown-none
```

This represents a freestanding x86_64 environment rather than a Linux userspace application.

## 3.4 Cargo

Cargo is used for:

- Package management.
- Dependency management.
- Workspace management.
- Compilation.
- Build automation.
- Reproducible builds.

## 3.5 Nightly Rust

Nightly Rust is used for the kernel project because the selected bare-metal and bootloader workflow depends on compiler/Cargo functionality that is not available entirely through the stable toolchain.

The working toolchain configuration shall be documented so the environment can be reproduced.

## 3.6 Bootloader

The Rust bootloader ecosystem is used instead of implementing the complete PC boot process from scratch.

This keeps the project focused on:

- Kernel development.
- Memory.
- Interrupts.
- Shell.
- Diagnostics.
- AI integration.

rather than spending most of the project duration on firmware and boot-protocol implementation.

## 3.7 QEMU

QEMU provides a safe and reproducible environment for kernel development.

Its main benefits are:

- No need to repeatedly reboot the host machine.
- Kernel crashes are isolated from the development OS.
- Hardware configuration is reproducible.
- Serial output can be connected to the host terminal.
- Debugging can be performed without risking the host system.

## 3.8 UART / Serial

Serial communication is selected because it provides a lightweight output and communication mechanism before a complete graphical or networking subsystem exists.

It can support:

- Early kernel debugging.
- Kernel logs.
- Panic diagnostics.
- Structured diagnostic transfer.
- Host-side communication.

## 3.9 `uart_16550`

The `uart_16550` crate provides an abstraction over a 16550-compatible UART, reducing the amount of hardware-register code that must be implemented manually during the initial kernel development stages.

## 3.10 `spin`

A spin-based lock is appropriate for early kernel synchronization where normal OS-level locking mechanisms are not yet available.

It can protect shared resources such as the global serial output device.

## 3.11 Host-side AI Bridge

The AI Bridge isolates the external AI dependency from the kernel.

This provides:

- Lower kernel complexity.
- Better fault isolation.
- Easier provider replacement.
- Easier AI testing.
- No need for networking in the kernel MVP.

## 3.12 REST / HTTPS

REST/HTTPS is used between the AI Bridge and the external AI provider because the host already has normal networking support.

Therefore:

```text
Kernel
  ↓
Serial
  ↓
AI Bridge
  ↓
HTTPS
  ↓
AI Service
```

rather than implementing:

```text
Kernel
  ↓
TCP/IP
  ↓
TLS
  ↓
HTTP
  ↓
AI API
```

inside the MVP kernel.

---

# 4. Kernel Technical Requirements

## 4.1 Boot

The kernel shall:

- Compile for x86_64 bare-metal.
- Produce a bootloader-compatible kernel image.
- Reach the configured kernel entry point.
- Initialize core subsystems in a defined order.
- Run continuously after initialization.

## 4.2 Interrupts and Exceptions

The kernel shall:

- Configure the required interrupt descriptor structures.
- Install supported exception handlers.
- Handle required hardware interrupts.
- Provide controlled behavior for supported exceptions.
- Generate diagnostics for relevant failures.

## 4.3 Memory Management

The kernel shall provide the memory functionality required by the MVP, including:

- Access to boot-time memory information.
- Required page-table/paging configuration.
- Kernel heap initialization.
- Dynamic allocation support when required.
- Controlled handling of allocation failures.

## 4.4 Keyboard Input

The kernel shall:

- Receive keyboard input events.
- Decode supported key events.
- Provide input to the shell layer.
- Support basic command-line interaction.

## 4.5 Serial Communication

The kernel shall:

- Initialize the configured UART.
- Transmit text and structured messages.
- Provide diagnostic output.
- Provide a defined boundary for host communication.

## 4.6 Logging

The kernel shall provide at least:

```text
INFO
WARN
ERROR
PANIC
```

Logging shall:

- Use a consistent format.
- Be available through serial output.
- Avoid exposing unnecessary sensitive information.
- Support future diagnostic processing.

## 4.7 Panic Handling

The panic mechanism shall:

- Capture available panic information.
- Record diagnostic information.
- Produce serial output.
- Enter a controlled halt/failure state for unrecoverable failures.

## 4.8 Shell

The shell shall provide:

- Command input.
- Command parsing.
- Command validation.
- Command dispatch.
- Command output.
- Controlled error messages.
- Basic command history where implemented.
- Basic command auto-completion where implemented.

---

# 5. Shell Technical Requirements

## 5.1 Command Processing Pipeline

```text
Keyboard Input
    ↓
Input Buffer
    ↓
Parser
    ↓
Validation
    ↓
Command Registry
    ↓
Command Handler
    ↓
Result
    ↓
Terminal Output
```

## 5.2 Command Categories

The initial shell may contain commands such as:

```text
help
clear
info
meminfo
panic-test
ai
```

The final command set may evolve during implementation.

## 5.3 AI Command Handling

AI-related shell requests shall:

1. Receive user input.
2. Parse the request.
3. Validate the request.
4. Package required context.
5. Send the request through the communication boundary.
6. Wait for a bounded response.
7. Validate the returned response.
8. Display the result.

---

# 6. Diagnostic Technical Requirements

## 6.1 Diagnostic Generation

The kernel shall generate structured diagnostics for selected events such as:

- Kernel panic.
- Memory failures.
- Unsupported exceptions.
- Initialization failures.
- Communication failures.
- Shell errors where applicable.

## 6.2 Diagnostic Record

A conceptual diagnostic record may contain:

```json
{
  "type": "diagnostic",
  "severity": "error",
  "subsystem": "memory",
  "message": "Heap allocation failed",
  "context": {}
}
```

The exact serialization format will be finalized during implementation.

## 6.3 Diagnostic Pipeline

```text
Kernel Event
    ↓
Diagnostic Record
    ↓
Logger
    ↓
Serial / Communication Boundary
    ↓
AI Bridge
    ↓
Diagnostic Processing
    ↓
AI Request
```

---

# 7. AI Bridge Technical Requirements

## 7.1 Responsibilities

The AI Bridge shall:

- Receive messages from Psydian.
- Parse protocol messages.
- Validate incoming data.
- Identify request type.
- Prepare AI context.
- Call the external AI service.
- Validate the returned response.
- Serialize the response.
- Return the response to Psydian.

## 7.2 Processing Pipeline

```text
Incoming Message
    ↓
Channel Reader
    ↓
Protocol Parser
    ↓
Message Validation
    ↓
Diagnostic / Request Processor
    ↓
Prompt Builder
    ↓
AI Client
    ↓
External AI
    ↓
Response Parser
    ↓
Response Validation
    ↓
Protocol Serializer
    ↓
Channel Writer
```

## 7.3 AI Provider Abstraction

The AI Bridge should isolate provider-specific implementation behind an internal interface.

Conceptually:

```text
AI Client Interface
        ↓
 ┌──────┼──────┐
 ↓      ↓      ↓
Provider A  Provider B  Local Model
```

The kernel must not depend on any particular AI provider.

---

# 8. Communication Protocol Requirements

The kernel and AI Bridge shall use a defined communication protocol rather than arbitrary raw text where structured diagnostics are required.

## 8.1 Message Categories

The MVP may support:

```text
AI_REQUEST
DIAGNOSTIC
AI_RESPONSE
ERROR
STATUS
```

## 8.2 Message Structure

Conceptually:

```json
{
  "type": "diagnostic",
  "id": "diag-001",
  "payload": {}
}
```

## 8.3 Protocol Requirements

The communication protocol shall:

- Identify message type.
- Clearly delimit messages.
- Validate message structure.
- Reject malformed messages.
- Prevent arbitrary data from being treated as a valid command.
- Support future protocol extensions.
- Provide enough information for debugging communication failures.

---

# 9. AI Response Requirements

The AI layer shall be used primarily for:

- System-diagnostic explanations.
- Troubleshooting assistance.
- Suggested investigation steps.
- Optional command suggestions.
- Natural-language explanations.

The AI shall not be treated as the authoritative system execution layer.

## 9.1 Response Processing

```text
AI Response
    ↓
Parse
    ↓
Schema Validation
    ↓
Safety Validation
    ↓
Display / Suggest
```

## 9.2 Suggested Commands

When the AI produces a command suggestion:

```text
AI
 ↓
Suggested Command
 ↓
Shell Parser
 ↓
Command Validation
 ↓
Risk Check
 ↓
User Confirmation if required
 ↓
Execution
```

AI output shall never bypass the shell command-validation layer.

---

# 10. Security Requirements

## 10.1 Trust Boundary

```text
TRUSTED
────────────────────────────
Psydian Kernel
       │
       │ controlled channel
       ▼
────────────────────────────
UNTRUSTED / EXTERNAL
AI Bridge
       │
       ▼
External AI Service
```

## 10.2 Security Requirements

- The external AI service shall not have direct kernel-memory access.
- The AI Bridge shall remain outside kernel privilege.
- AI output shall be treated as untrusted.
- Suggested commands shall undergo normal validation.
- Privileged/destructive operations shall require confirmation.
- API credentials shall remain outside kernel source code.
- Secrets shall not be committed to Git.
- Diagnostic data sent externally shall be limited to required information.
- Malformed protocol messages shall be rejected.
- External service failures shall not directly compromise kernel execution.

---

# 11. Reliability Requirements

## 11.1 Core Reliability

- Kernel initialization should be deterministic in the supported QEMU environment.
- Supported exceptions should have defined handlers.
- Panic conditions should enter controlled failure states.
- Logging should remain available during diagnostic scenarios.
- Communication failures should be isolated from kernel stability.

## 11.2 AI Reliability

- AI requests shall use bounded timeouts.
- AI service failures shall produce controlled errors.
- Malformed AI responses shall be rejected.
- AI unavailability shall not disable normal shell operation.
- The bridge should be capable of continuing to process future requests after recoverable failures.

---

# 12. Performance Requirements

## 12.1 Kernel

The MVP should prioritize deterministic and responsive local interactions.

Requirements:

- Shell commands should respond without unnecessary delay.
- Serial logging should avoid excessive blocking.
- Kernel initialization should avoid unnecessary work.
- Memory-management operations should remain appropriate for the QEMU test environment.

## 12.2 AI

AI latency is an external dependency.

Therefore:

- AI requests shall not block the kernel indefinitely.
- Timeouts shall be defined.
- Local kernel operation shall remain independent of AI latency.
- AI response time should be measured during testing.

---

# 13. Resource Requirements

## 13.1 Development Hardware

The project requires a development machine capable of running:

- Linux host OS.
- Rust toolchain.
- QEMU.
- Build tools.
- AI Bridge.
- Network-enabled AI API access.

## 13.2 Virtual Machine

QEMU shall be configured with sufficient:

- RAM.
- CPU resources.
- Serial device support.
- Boot image support.

The exact configuration may be adjusted during testing.

## 13.3 External AI

An external AI endpoint requires:

- Network connectivity from the host.
- Valid provider credentials.
- API access.
- Appropriate request limits.

---

# 14. Build Requirements

## 14.1 Toolchain

The working project environment shall include:

```text
Rust nightly
Cargo
rust-src
llvm-tools-preview
x86_64-unknown-none
QEMU
Git
```

## 14.2 Kernel Build

The kernel shall be compiled with:

```text
x86_64-unknown-none
```

and shall use:

```rust
#![no_std]
#![no_main]
```

## 14.3 Boot Image

The build pipeline shall produce a bootable BIOS image for the initial QEMU MVP.

Conceptually:

```text
Kernel Binary
     ↓
Bootloader
     ↓
BIOS Disk Image
     ↓
QEMU
```

---

# 15. Repository Requirements

The implementation should maintain clear separation between documentation and code.

```text
PsyDian_OS/
├── Docs/
│   ├── Complete-Documentation.md
│   ├── HLD.md
│   ├── LLD.md
│   └── OJT-PRD-SEM-3(8-week).pdf
│
├── kernel/
│   ├── Cargo.toml
│   └── src/
│
├── src/
│
├── build.rs
├── Cargo.toml
├── Cargo.lock
├── .cargo/
│   └── config.toml
├── rust-toolchain.toml
└── README.md
```

The exact source structure may evolve as implementation progresses, while preserving the module boundaries defined in the LLD.

---

# 16. Testing Requirements

The technical implementation shall be tested incrementally.

## Kernel Tests

Validate:

- Boot success.
- Kernel entry.
- Serial output.
- Logging.
- Exception handling.
- Interrupt handling.
- Keyboard input.
- Memory initialization.
- Allocation behavior.
- Panic behavior.
- Diagnostic generation.

## AI Bridge Tests

Validate:

- Message parsing.
- Protocol validation.
- Diagnostic processing.
- AI request creation.
- AI response parsing.
- AI response validation.
- Timeout handling.
- External API failure.
- Malformed response handling.

## Integration Tests

Validate the complete path:

```text
User
 ↓
Shell
 ↓
Kernel
 ↓
Diagnostic Channel
 ↓
AI Bridge
 ↓
External AI
 ↓
AI Bridge
 ↓
Kernel
 ↓
Shell
 ↓
User
```

---

# 17. NFR Targets

## Reliability

- Core system operation must not depend on AI availability.
- Defined panic paths shall be reproducible.
- Communication failures shall be controlled.

## Security

- No direct AI access to kernel memory.
- No automatic execution of arbitrary AI-generated privileged commands.
- No committed API secrets.

## Maintainability

- Modules shall have clear responsibilities.
- Kernel and AI Bridge shall remain separate.
- Provider-specific AI code shall be isolated.

## Testability

- Major kernel subsystems shall have reproducible validation scenarios.
- Protocol behavior shall be independently testable.
- AI failures shall be testable without depending exclusively on live API calls.

## Performance

- Local shell interactions should remain responsive.
- External AI latency shall be bounded using timeouts.
- Serial logging should not excessively interfere with normal kernel operation.

## Portability

- MVP target: x86_64 + QEMU.
- Hardware-specific functionality should be isolated where practical.
- Future hardware support should not require redesigning unrelated layers.

## Observability

- Important kernel events shall be logged.
- Panic diagnostics shall be captured.
- AI Bridge failures shall be distinguishable from kernel failures.
- Communication failures shall provide sufficient debugging context.

## Privacy

- Only required diagnostic information should cross the external AI boundary.
- Sensitive information should not be unnecessarily logged.
- Credentials shall remain outside source code.

## Project Constraints

- MVP implementation shall fit within the 8-week project period.
- Core OS functionality takes priority over optional voice functionality.
- Advanced networking, filesystem, GUI, multi-user support, and other large subsystems remain outside the core MVP.

