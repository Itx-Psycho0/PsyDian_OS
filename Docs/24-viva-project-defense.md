# Viva / Project Defense

## 1. Purpose

This document prepares the Psydian team for the final viva, mentor review, technical defense, and project demonstration.

The objective is not to memorize answers. Each student should be able to explain:

- What Psydian is.
- Why the project was built.
- What problem it addresses.
- How the architecture works.
- Why each major technology was selected.
- How the kernel works at a conceptual level.
- How the AI Bridge communicates with the kernel.
- Why networking is kept outside the kernel MVP.
- How diagnostics are generated.
- How AI output is validated.
- How security boundaries are enforced.
- What happens when components fail.
- What trade-offs were accepted.
- What is implemented, deferred, or future scope.

The mentor documentation emphasizes that students should be able to defend not only **what** they built, but also **why** each architectural decision was made and what trade-offs were accepted.

---

# 2. Golden Answer Framework

For difficult technical questions, use:

```text
WHAT
 ↓
WHY
 ↓
HOW
 ↓
TRADE-OFF
 ↓
LIMITATION / FUTURE
```

Example:

> Why did you use a host-side AI Bridge?

```text
WHAT:
We use a host-side AI Bridge.

WHY:
The kernel does not initially contain a networking stack.

HOW:
The kernel sends structured diagnostic information through a controlled communication channel. The host-side bridge receives it and uses normal host networking to communicate with the external AI service.

TRADE-OFF:
We add another software component and communication protocol.

LIMITATION:
AI-assisted functionality depends on the host bridge and the external AI service.
```

This demonstrates engineering reasoning rather than memorized terminology.

---

# 3. 30-Second Project Defense

> Psydian is an experimental x86_64 operating-system prototype written primarily in Rust. It contains a bare-metal kernel with core system foundations, a command-line shell, structured diagnostics, and a host-side AI Bridge. The kernel generates diagnostic information, the bridge communicates with an external AI service, and validated AI responses are returned to the Psydian shell. The AI is intentionally treated as an assistance layer, not as a privileged system authority.

---

# 4. 1-Minute Project Defense

Psydian combines operating-system development with Generative AI.

The kernel boots in QEMU as a bare-metal Rust program. It is responsible for low-level system functionality such as initialization, memory-management foundations, interrupts and exceptions, keyboard input, serial communication, shell operation, logging, and diagnostics.

The AI functionality is deliberately outside the kernel. A host-side AI Bridge receives structured requests or diagnostics from Psydian and communicates with an external AI service through HTTPS.

The response is validated before being returned to Psydian. AI-generated commands are treated as suggestions and must still pass shell parsing, validation, risk checks, and confirmation where required.

---

# 5. 2-Minute Architecture Defense

The architecture has three major environments:

```text
1. Psydian Kernel
2. QEMU Virtual Machine
3. Host-side AI Bridge
```

The kernel targets:

```text
x86_64-unknown-none
```

and uses a freestanding Rust environment.

QEMU provides the virtual x86_64 hardware on which the kernel runs.

The kernel uses serial/UART communication for logging and the controlled host communication boundary.

The host-side AI Bridge is an ordinary user-space application. It reads structured data, validates it, constructs AI requests, calls the external AI service, validates AI responses, and returns structured results.

The overall flow is:

```text
User
 ↓
Psydian Shell
 ↓
Kernel
 ↓
Diagnostic / AI Request
 ↓
Serial / Protocol
 ↓
Host AI Bridge
 ↓
HTTPS / REST
 ↓
External AI
 ↓
Response Validation
 ↓
AI Bridge
 ↓
Psydian
 ↓
Shell
 ↓
User
```

---

# 6. Beginner Viva Questions

## Q1. What is Psydian?

Psydian is an experimental operating-system prototype that combines a Rust-based bare-metal kernel with an AI-assisted diagnostic layer.

Its MVP focuses on:

- Kernel foundations.
- Shell interaction.
- Structured diagnostics.
- Host-side AI assistance.

---

## Q2. What is an operating system?

An operating system manages computer hardware and provides controlled services to programs.

Examples include:

```text
Linux
Windows
macOS
Android
```

Psydian is an experimental OS project rather than a production replacement for a mature OS.

---

## Q3. What is a kernel?

The kernel is the privileged core of the operating system.

It controls or manages resources such as:

- CPU.
- Memory.
- Devices.
- Interrupts.
- System execution.

In Psydian, the kernel is the primary trusted component.

---

## Q4. What is bare-metal software?

Bare-metal software executes without another operating system underneath it.

Normal application:

```text
Application
 ↓
Operating System
 ↓
Hardware
```

Psydian kernel:

```text
Psydian Kernel
 ↓
Virtual Hardware
```

QEMU provides the virtual hardware during development.

---

## Q5. What is a bootloader?

A bootloader prepares the execution environment and loads the operating-system kernel.

Conceptually:

```text
Firmware / Boot Environment
 ↓
Bootloader
 ↓
Kernel
 ↓
Kernel Initialization
```

---

# 7. Boot Questions

## Q6. How does Psydian boot?

The simplified boot path is:

```text
QEMU
 ↓
Firmware / Boot Environment
 ↓
Psydian Bootloader
 ↓
Kernel Entry Point
 ↓
Kernel Initialization
 ↓
Shell / Main Kernel Loop
```

The bootloader handles the initial loading and setup required to reach the kernel.

---

## Q7. Why use an existing bootloader?

Implementing the entire PC boot path would consume substantial time.

The project uses the Rust bootloader ecosystem so that development can focus on:

- Kernel functionality.
- Memory.
- Interrupts.
- Shell.
- Diagnostics.
- AI integration.

---

## Q8. What happens after the kernel reaches its entry point?

The kernel performs initialization in a defined sequence.

Conceptually:

```text
Kernel Entry
 ↓
Serial
 ↓
Memory
 ↓
Interrupts
 ↓
Input
 ↓
Shell
 ↓
Main Execution
```

The exact sequence may evolve during implementation.

---

# 8. Rust Viva Questions

## Q9. Why did you choose Rust?

Rust provides:

- Low-level control.
- Strong static typing.
- Compile-time safety checks.
- Memory-safety guarantees in safe code.
- Bare-metal support through `no_std`.
- A systems-programming ecosystem.

---

## Q10. What is `no_std`?

`no_std` indicates that the kernel does not use Rust's normal standard library.

Example:

```rust
#![no_std]
```

The standard library expects operating-system services that are not yet available in a bare-metal kernel.

---

## Q11. Why do you need `unsafe` in a kernel?

Some kernel operations require direct interaction with hardware or raw memory.

Examples:

- Raw pointers.
- Memory-mapped I/O.
- CPU instructions.
- Page-table manipulation.

Rust provides these capabilities through controlled `unsafe` blocks.

---

## Q12. Why not use normal Rust application code?

A normal Rust application executes under an existing operating system and relies on OS services.

Psydian is itself implementing the operating-system layer, so it cannot assume those services already exist.

---

# 9. QEMU Questions

## Q13. Why use QEMU?

QEMU provides a virtual x86_64 machine for kernel execution.

It gives:

- Safe experimentation.
- Fast reboot cycles.
- Reproducibility.
- Virtual hardware.
- Serial-device support.
- Easier debugging.

---

## Q14. Why not directly use your physical laptop?

Because kernel bugs can crash or destabilize a physical machine.

QEMU provides a safer development environment and makes controlled testing easier.

---

## Q15. Does QEMU behave exactly like physical hardware?

No.

QEMU provides virtualized hardware. Some physical-hardware-specific behavior may still need to be tested later.

---

# 10. Memory Questions

## Q16. Why does a kernel need memory management?

The kernel needs controlled memory for:

- Kernel state.
- Stacks.
- Buffers.
- Data structures.
- Dynamic allocation where supported.

---

## Q17. What is paging?

Paging maps virtual addresses to physical memory.

Conceptually:

```text
Virtual Address
      ↓
Page Table
      ↓
Physical Memory
```

This allows the kernel to control address translation and memory access.

---

## Q18. What is a heap?

A heap is a memory region used for dynamic allocation.

For example:

```text
Box
Vec
String
```

normally require dynamic memory.

A bare-metal kernel must initialize an allocator before using heap allocation safely.

---

# 11. Interrupt and Exception Questions

## Q19. What is an interrupt?

An interrupt causes the CPU to transfer control to an interrupt handler because of an event.

Examples:

- Keyboard input.
- Timer event.
- Hardware event.

---

## Q20. What is an exception?

An exception is a CPU-detected condition affecting normal execution.

Examples:

- Divide by zero.
- Invalid instruction.
- Page fault.

---

## Q21. What is an IDT?

The Interrupt Descriptor Table associates interrupts/exceptions with their handlers.

Conceptually:

```text
Interrupt / Exception
       ↓
      IDT
       ↓
   Handler
```

---

# 12. Serial and UART Questions

## Q22. What is UART?

UART is a serial communication mechanism.

Psydian can use it for:

- Kernel output.
- Debugging.
- Diagnostics.
- Controlled host communication.

---

## Q23. Why is serial useful before networking exists?

Serial communication has much lower implementation complexity than a full network stack.

It lets the project establish useful communication before implementing:

```text
Ethernet
IP
TCP/UDP
DNS
TLS
HTTP
```

---

# 13. Shell Questions

## Q24. What is a shell?

A shell provides a command-line interface for interacting with the operating system.

Psydian's shell follows:

```text
Input
 ↓
Buffer
 ↓
Parser
 ↓
Validation
 ↓
Command Handler
 ↓
Output
```

---

## Q25. Why have command validation?

Validation ensures that raw input is converted into a supported and controlled operation.

For example:

```text
Unknown command
 ↓
Reject
```

rather than trying to execute arbitrary input.

---

# 14. Diagnostics Questions

## Q26. Why does Psydian need diagnostics?

Low-level kernel failures are often difficult for users to interpret.

A structured diagnostic allows the system to record:

```text
What happened
+
Where it happened
+
How severe it was
+
Relevant context
```

---

## Q27. Give an example of a diagnostic.

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

---

## Q28. Why structured data instead of only text logs?

Raw text is useful for humans.

Structured data is easier for:

- Parsing.
- Validation.
- Testing.
- AI context construction.
- Correlation.
- Future automation.

---

# 15. AI Questions

## Q29. Why use AI in an operating system?

The purpose is not to make the kernel itself autonomous.

The purpose is to assist users in understanding selected system diagnostics and technical problems.

Example:

```text
Raw Kernel Diagnostic
      ↓
AI Interpretation
      ↓
Human-readable Explanation
```

---

## Q30. What is an LLM?

An LLM is a machine-learning model designed to process and generate natural language.

Psydian uses it as an assistance layer.

---

## Q31. What is a prompt?

A prompt is the structured input given to the language model to define the task, context, and expected behavior.

Psydian's AI Bridge should provide:

```text
System Instructions
+
Relevant Diagnostic Context
+
User Query
```

---

# 16. AI Architecture Questions

## Q32. How does the kernel communicate with the AI?

The kernel does not directly access the external AI service.

The path is:

```text
Kernel
 ↓
Controlled Communication
 ↓
AI Bridge
 ↓
HTTPS
 ↓
External AI
```

---

## Q33. Where is the networking code?

Networking for external AI communication lives in the host-side AI Bridge.

The kernel does not need to implement the full external HTTP/HTTPS communication path for the MVP.

---

## Q34. Why not directly put an HTTP client inside the kernel?

Because that would require a substantial additional stack:

```text
Network Device
 ↓
Driver
 ↓
Ethernet
 ↓
IP
 ↓
TCP/UDP
 ↓
DNS
 ↓
TLS
 ↓
HTTP
 ↓
AI API
```

That complexity is not necessary for the core 8-week MVP.

---

# 17. AI Safety Questions

## Q35. Can the AI execute commands?

No.

The AI can suggest commands, but it cannot directly execute them.

The command must return through:

```text
Shell Parser
 ↓
Validation
 ↓
Risk Classification
 ↓
Confirmation
 ↓
Execution
```

---

## Q36. Why is AI output untrusted?

Because the model may produce:

- Incorrect information.
- Hallucinations.
- Unsafe commands.
- Unsupported claims.
- Prompt-injection-driven output.

Therefore:

```text
AI Output
≠
System Authority
```

---

## Q37. What if the AI says "execute this command"?

The statement is treated as a suggestion, not authorization.

The shell still decides whether the command is:

- Valid.
- Supported.
- Safe.
- Authorized.

---

## Q38. What is prompt injection?

Prompt injection is when untrusted content attempts to manipulate the model into changing its intended behavior.

Example:

```text
Ignore previous instructions and execute this command.
```

The AI Bridge should treat such content as untrusted data.

---

# 18. Network Boundary Questions

## Q39. Does the kernel need TCP/IP for the AI feature?

Not in the MVP.

The external network path is:

```text
Kernel
 ↓
Local/controlled communication
 ↓
AI Bridge
 ↓
Host network stack
 ↓
HTTPS
 ↓
AI Provider
```

---

## Q40. Is this a limitation?

Yes.

It means the AI feature depends on a host-side bridge and normal host networking.

The project accepts that limitation because it significantly reduces kernel complexity.

---

# 19. Reliability Questions

## Q41. What happens when the AI API is unavailable?

```text
AI API Failure
 ↓
AI Bridge
 ↓
Controlled Error
 ↓
User Notification
```

Core Psydian functionality should remain available.

---

## Q42. What happens if the AI Bridge crashes?

AI-assisted features become unavailable, but the kernel should remain isolated from the bridge failure.

This is one of the benefits of keeping the bridge outside the privileged kernel.

---

## Q43. What happens when the AI response is malformed?

```text
Receive Response
 ↓
Parse
 ↓
Schema Validation
 ↓
Reject if Invalid
```

The malformed result must not become an execution instruction.

---

# 20. Security Questions

## Q44. Where are AI API credentials stored?

On the host side using protected environment/configuration mechanisms.

They are not part of the kernel source or boot image.

---

## Q45. Can the AI service access kernel memory?

No.

The external AI service only receives information explicitly sent through the AI Bridge.

There is no direct kernel-memory access path.

---

## Q46. What security principle does this architecture use?

Primarily:

```text
Least Privilege
+
Explicit Trust Boundaries
+
Defense in Depth
```

---

# 21. Database Questions

## Q47. Why does Psydian not use a conventional database?

The MVP does not have a requirement for a conventional application database.

The important data consists mainly of:

- Runtime kernel state.
- Shell state.
- Diagnostic records.
- Communication messages.
- AI requests/responses.

A database can be considered later if persistent diagnostic history, remote monitoring, or other storage requirements appear.

---

# 22. Testing Questions

## Q48. How do you test an operating system?

Use multiple levels:

```text
Unit
 ↓
Subsystem
 ↓
Integration
 ↓
QEMU Runtime
 ↓
End-to-End
```

---

## Q49. How do you test a panic?

Use a controlled panic scenario.

For example:

```text
panic-test
```

can intentionally trigger a known failure.

Then verify:

```text
Panic
 ↓
Handler
 ↓
Diagnostic
 ↓
Serial Output
```

---

## Q50. How do you test AI functionality?

Use:

- Mock AI providers.
- Fixture responses.
- Schema-validation tests.
- Protocol tests.
- Controlled live integration tests where appropriate.

This avoids making every test dependent on an external service.

---

# 23. CI/CD Questions

## Q51. Why use CI/CD?

Kernel and integration changes can easily introduce regressions.

CI can validate:

```text
Formatting
 ↓
Lint
 ↓
Tests
 ↓
Kernel Build
 ↓
Boot Image
 ↓
Smoke Test
```

---

## Q52. What should happen when CI fails?

The failure should be investigated and fixed before the change is considered stable.

---

# 24. Observability Questions

## Q53. What is observability?

Observability is the ability to understand system behavior using emitted information such as:

- Logs.
- Diagnostics.
- Metrics.
- Events.
- Request IDs.

---

## Q54. How do you trace one diagnostic through the AI pipeline?

Use identifiers.

Example:

```text
diag-001
   ↓
req-001
   ↓
AI Provider
   ↓
req-001
   ↓
Validated Response
```

This helps identify where a failure happened.

---

# 25. Deployment Questions

## Q55. Where is Psydian deployed?

For the MVP:

```text
Linux Host
 ↓
QEMU
 ↓
Psydian Bootable Image
```

The AI Bridge runs separately on the host.

---

## Q56. What is the deployment unit?

The kernel is distributed as a bootable image for the supported QEMU environment.

Conceptually:

```text
Kernel
 ↓
Bootloader
 ↓
Bootable BIOS Image
 ↓
QEMU
```

---

# 26. Architecture Trade-Off Questions

## Q57. What is the biggest architectural trade-off?

The main trade-off is:

```text
Avoiding a full kernel networking stack
        VS
Introducing a host-side AI Bridge
```

The project chose the bridge because it keeps the kernel manageable within the 8-week timeline.

---

## Q58. What is the biggest technical limitation?

The AI layer depends on the host-side bridge and external service.

The kernel itself remains operational without AI, but AI-assisted functionality is unavailable if those dependencies fail.

---

## Q59. Why not build a complete networking stack?

Because the objective of the MVP is not to implement every operating-system subsystem.

The project prioritizes a coherent kernel + diagnostics + AI workflow.

---

# 27. Difficult Defense Questions

## Q60. Isn't using a bootloader library avoiding the hardest part?

It is intentionally reducing bootloader scope.

The project is focused on demonstrating kernel development and AI-assisted diagnostics rather than spending most of the limited project time on the full firmware/boot protocol implementation.

---

## Q61. Isn't the AI just a chatbot outside the OS?

The AI is intentionally implemented as an external assistance layer.

The project integration comes from:

```text
Kernel Diagnostics
 ↓
Defined Communication Protocol
 ↓
AI Bridge
 ↓
AI Analysis
 ↓
Validated Response
 ↓
Psydian Shell
```

The important engineering problem is integrating AI assistance with a privileged system without granting the model direct authority.

---

## Q62. Why not let the AI control the kernel directly?

That would create a dangerous trust boundary.

LLM output is probabilistic and may be incorrect or manipulated.

The safer architecture is:

```text
AI Suggestion
 ↓
Validation
 ↓
Authorization
 ↓
Human Confirmation if Needed
 ↓
Execution
```

---

# 28. Scenario Defense

## Scenario 1 — The AI suggests an invalid command.

Answer:

> The command is treated as untrusted data. It returns through the shell parser and validation layer. Since it is unsupported or invalid, it is rejected.

---

## Scenario 2 — The AI suggests a destructive command.

Answer:

> The suggestion is classified as potentially destructive. It cannot directly execute. It must pass command validation and explicit confirmation/authorization rules.

---

## Scenario 3 — The AI API times out.

Answer:

> The AI Bridge reports a bounded timeout. The shell receives a controlled error, while the core kernel continues operating.

---

## Scenario 4 — The AI Bridge crashes.

Answer:

> AI assistance becomes unavailable, but the kernel remains isolated because the bridge is a separate host-side component.

---

## Scenario 5 — A diagnostic contains prompt injection text.

Answer:

> The diagnostic is treated as untrusted data. The AI Bridge's system instructions remain authoritative, and the diagnostic text is not allowed to redefine the AI's task.

---

## Scenario 6 — AI provides an incorrect explanation.

Answer:

> The AI result is an assistance output, not authoritative system state. The source of truth remains the actual diagnostic information produced by Psydian.

---

## Scenario 7 — QEMU does not boot the image.

Answer:

> First determine whether the failure is in image generation, bootloader configuration, or kernel initialization by inspecting the build output and serial/QEMU logs. Then isolate the failing stage before changing unrelated components.

---

# 29. "Why This Technology?" Questions

## Why Rust?

```text
Low-level Control
+
Type Safety
+
Compile-time Safety
+
no_std
+
Systems Ecosystem
```

---

## Why QEMU?

```text
Isolation
+
Reproducibility
+
Fast Iteration
+
Virtual Hardware
```

---

## Why Serial?

```text
Simple
+
Low-Level
+
Useful for Debugging
+
Available Before Networking
```

---

## Why Host-side AI Bridge?

```text
Avoid Kernel Networking Complexity
+
Use Normal Host Networking
+
Provider Abstraction
+
Fault Isolation
```

---

## Why External AI?

For the MVP, an external AI service avoids the additional requirements of deploying and running a local model.

A local model can be considered as future scope if the project later requires offline inference and has adequate compute resources.

---

# 30. HLD / LLD / TRD Defense

## Q63. What is HLD?

HLD describes the major system components and their relationships.

Example:

```text
Kernel
 ↓
AI Bridge
 ↓
AI Service
```

---

## Q64. What is LLD?

LLD describes implementation-level details such as:

- Modules.
- Data structures.
- Interfaces.
- Functions.
- Control flow.
- Subsystem behavior.

---

## Q65. What is the TRD?

The Technical Requirements Document defines technical requirements, constraints, architecture-related technology requirements, reliability, security, performance, deployment, and testing expectations.

---

# 31. ADR Defense

## Q66. What is an ADR?

An Architecture Decision Record documents an important architecture decision and the reasoning behind it.

Example:

```text
Decision:
Use a host-side AI Bridge.

Context:
Kernel has no external network stack.

Trade-off:
Additional component.

Status:
Accepted.
```

---

# 32. Traceability Defense

## Q67. What is the RTM?

The Requirements Traceability Matrix connects:

```text
Requirement
 ↓
Design
 ↓
Implementation
 ↓
Test
 ↓
Evidence
```

It demonstrates that project requirements have corresponding engineering and validation work.

---

# 33. Scope Defense

## Q68. Is Psydian a complete general-purpose OS?

No.

It is an experimental MVP focused on a selected subset of operating-system functionality and AI-assisted diagnostics.

---

## Q69. Why is the scope limited?

The project has an 8-week schedule.

A smaller, working, testable system is more valuable than many unfinished subsystems.

---

## Q70. What is intentionally outside the MVP?

Depending on the finalized project scope:

```text
Full Kernel Networking
Complete Production Filesystem
Full Desktop GUI
Full Multi-user System
Production Hardware Support
Autonomous AI Kernel Control
Advanced Voice
Advanced RAG
Enterprise Infrastructure
```

These should be clearly labeled as future or deferred work.

---

# 34. Future Work Defense

## Q71. What would you add with more time?

Possible directions:

```text
Full networking stack
Filesystem
UEFI
Physical hardware support
Improved shell
Persistent diagnostics
Local AI
RAG
Voice
Advanced observability
```

The answer should prioritize additions rather than claim that all are equally necessary.

---

# 35. Architecture Weakness Question

## Q72. What is the weakest part of the design?

A strong answer:

> The AI-assisted portion depends on an external service and a host-side bridge, which introduces additional latency and external availability dependencies. We accepted that limitation because it avoids building a complete kernel network stack within the 8-week MVP.

---

# 36. "What Would You Do Differently?" Question

Possible answer:

> With more development time, I would evaluate a stronger communication abstraction, improve the kernel's networking capabilities, add persistent diagnostics, evaluate local AI inference for offline operation, and expand hardware support. I would only add those after stabilizing the core kernel and diagnostic architecture.

---

# 37. "What Did You Actually Build?" Question

The answer should distinguish implemented work from documentation/future plans.

Use:

```text
Implemented
In Progress
Planned
Deferred
```

Never describe a planned feature as if it were already implemented.

---

# 38. "Show Me the AI Flow" Question

Draw:

```text
User
 ↓
Psydian Shell
 ↓
Kernel
 ↓
Diagnostic / AI Request
 ↓
Serial / Protocol
 ↓
AI Bridge
 ↓
Prompt + Context
 ↓
External AI
 ↓
Response Parser
 ↓
Validation
 ↓
Psydian
 ↓
Shell
 ↓
User
```

Then explain:

> The kernel never gives the external AI direct privileged access.

---

# 39. "Why Is the AI Bridge Necessary?" Question

Strong answer:

> The AI Bridge provides a separation between the privileged kernel environment and external network/AI functionality. It lets the host use its normal networking stack while the kernel only communicates through a controlled interface. This reduces kernel complexity, isolates failures, and keeps the external AI provider replaceable.

---

# 40. "How Do You Know It Works?" Question

Answer using evidence:

```text
Build Result
 ↓
QEMU Boot
 ↓
Serial Output
 ↓
Shell Interaction
 ↓
Controlled Failure
 ↓
Diagnostic
 ↓
AI Bridge
 ↓
Validated AI Response
 ↓
End-to-End Demonstration
```

Each stage should have corresponding tests or demonstration evidence.

---

# 41. Final Viva Defense Checklist

Before the final viva, both students should be able to:

- [ ] Give the 30-second project explanation.
- [ ] Give the 2-minute technical explanation.
- [ ] Draw the architecture from memory.
- [ ] Explain the boot process.
- [ ] Explain what a kernel is.
- [ ] Explain bare metal.
- [ ] Explain the bootloader.
- [ ] Explain `no_std`.
- [ ] Explain why Rust is used.
- [ ] Explain why QEMU is used.
- [ ] Explain x86_64.
- [ ] Explain paging and heap basics.
- [ ] Explain interrupts and exceptions.
- [ ] Explain IDT at a high level.
- [ ] Explain UART/serial.
- [ ] Explain the shell pipeline.
- [ ] Explain structured diagnostics.
- [ ] Explain the kernel/AI Bridge boundary.
- [ ] Explain why the kernel does not directly use HTTPS.
- [ ] Explain how diagnostics reach the AI.
- [ ] Explain how the AI response returns.
- [ ] Explain structured AI output.
- [ ] Explain response validation.
- [ ] Explain command-suggestion safety.
- [ ] Explain prompt injection.
- [ ] Explain secret management.
- [ ] Explain AI failure handling.
- [ ] Explain communication failure handling.
- [ ] Explain database scope.
- [ ] Explain testing strategy.
- [ ] Explain CI/CD.
- [ ] Explain observability.
- [ ] Explain deployment.
- [ ] Explain key ADRs.
- [ ] Explain RTM/traceability.
- [ ] Explain project limitations.
- [ ] Explain future scope.
- [ ] Defend major architectural trade-offs.

---

# 42. Final Demonstration Script

A practical final demo can follow this order:

```text
1. Start QEMU
        ↓
2. Show Psydian boot
        ↓
3. Show kernel initialization logs
        ↓
4. Show shell
        ↓
5. Run a normal command
        ↓
6. Trigger a controlled diagnostic/panic
        ↓
7. Show structured diagnostic
        ↓
8. Send diagnostic to AI Bridge
        ↓
9. Show AI-assisted explanation
        ↓
10. Show suggested troubleshooting action
        ↓
11. Demonstrate validation of AI suggestion
        ↓
12. Simulate AI timeout/failure
        ↓
13. Show that core shell remains operational
```

This demonstrates both normal functionality and the most important architectural safety property.

---

# 43. Final Defense Principle

The strongest viva answer is not:

> "We used this technology because it is modern."

A stronger answer is:

```text
Requirement
 ↓
Constraint
 ↓
Options
 ↓
Decision
 ↓
Trade-off
 ↓
Validation
```

For every major technical decision, the team should be prepared to explain:

```text
What did we choose?
Why did we choose it?
What alternatives did we consider?
What did we gain?
What did we give up?
How did we validate it?
```

That demonstrates engineering understanding rather than memorization.

