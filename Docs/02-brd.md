# Business Requirements Document (BRD)

## 1. Executive Summary

Psydian is an AI-assisted operating-system prototype designed to provide a more intelligent and interactive system experience while demonstrating core operating-system and systems-programming concepts. The project focuses on building a minimal x86_64 operating-system environment in Rust, running it through QEMU, and providing users with a command-line interface supported by system diagnostics and external AI assistance.

The system is intended to help users understand and interact with operating-system behavior through a combination of traditional command-line interaction and AI-assisted troubleshooting. Users can execute supported system commands normally, while diagnostic information such as kernel logs and panic information can be passed through a controlled communication channel to a host-side AI Bridge. The AI Bridge communicates with an external AI service and returns explanations or recommendations to the system interface.

Psydian is designed as an assistance system rather than an autonomous operating system. The kernel remains independent from the external AI service, AI-generated responses are treated as untrusted information, and potentially privileged or destructive actions require normal validation and explicit user confirmation.

The project will be developed incrementally over an 8-week period, prioritizing a functional kernel foundation, shell interaction, diagnostics, AI-assisted analysis, reliability, security, and a demonstrable end-to-end workflow.

## 2. Problem Statement

Traditional command-line operating systems require users to understand commands, system terminology, error messages, and diagnostic information in order to troubleshoot problems effectively. When a kernel-level or system-level failure occurs, the information presented to the user may be technical and difficult to interpret, particularly for users who are still developing systems-programming knowledge.

At the same time, modern AI systems are capable of explaining technical information and generating troubleshooting guidance, but they are normally external to the operating-system execution environment. Directly embedding external AI communication into a bare-metal kernel would introduce unnecessary complexity, networking dependencies, and security risks.

Psydian addresses this problem by combining a minimal Rust-based operating-system environment with a separate AI assistance layer. The system can generate structured diagnostics, transfer selected information through a controlled communication channel, allow a host-side AI Bridge to communicate with an external AI service, and return the resulting explanation to the user.

## 3. Vision

Build a lightweight and modular operating-system prototype that combines traditional low-level operating-system functionality with an AI-assisted troubleshooting experience.

The long-term vision is for Psydian to provide a system interface where users can interact with the operating system naturally, understand system behavior through contextual explanations, and receive intelligent diagnostic assistance without giving an external AI direct control over the privileged kernel.

## 4. Objectives

- Build a bootable x86_64 operating-system prototype in Rust.
- Run and validate the system in QEMU.
- Implement the core kernel foundation required by the MVP.
- Provide an interactive command-line shell.
- Implement kernel logging and structured diagnostics.
- Implement controlled panic and exception diagnostics.
- Establish a controlled communication channel between Psydian and the host environment.
- Build a host-side AI Bridge for AI communication.
- Integrate an external AI service through the host system's network stack.
- Generate AI-assisted explanations of system diagnostics.
- Validate AI-generated responses before presenting or considering suggested actions.
- Keep core operating-system functionality independent of AI availability.
- Maintain separation between trusted kernel functionality and untrusted AI-generated content.
- Provide an architecture that can be extended with future operating-system and AI capabilities.

## 5. Personas

| Persona | Goals | Pain Points |
|---|---|---|
| Technical User | Interact with the system and understand system behavior | Technical errors and diagnostics may be difficult to interpret |
| Developer / Systems Programmer | Debug and understand low-level system behavior | Kernel failures require manual analysis and low-level debugging knowledge |
| Student / Learner | Learn operating-system concepts through practical interaction | Low-level concepts can be difficult to understand from raw output alone |
| Power User | Perform system operations and obtain contextual troubleshooting help | Existing command-line tools provide information but limited contextual explanation |

## 6. Business Use Cases

- Boot Psydian in a controlled x86_64 virtual environment.
- Execute supported shell commands.
- View kernel status and diagnostic information.
- Generate controlled kernel panic events for testing.
- Capture structured diagnostic information.
- Request AI-assisted explanation of system diagnostics.
- Send selected diagnostics to the host-side AI Bridge.
- Obtain an explanation or troubleshooting recommendation from the external AI service.
- Display the returned AI response through the Psydian shell.
- Validate AI-generated suggestions before considering execution.
- Require explicit user confirmation for privileged or destructive actions.
- Continue normal system operation when AI assistance is unavailable.
- Use optional voice interaction through the same existing shell/AI pipeline.

## 7. Business Requirements

| ID | Requirement | Priority | Acceptance Criteria |
|---|---|---|---|
| BR-001 | Psydian shall boot as an x86_64 kernel in the supported QEMU environment. | Must | Kernel reaches its entry point and produces the expected boot output. |
| BR-002 | Users shall be able to interact with Psydian through a command-line shell. | Must | Supported commands can be entered, parsed, executed, and reported. |
| BR-003 | Psydian shall provide kernel logging. | Must | Kernel events can be emitted through the defined logging mechanism. |
| BR-004 | Psydian shall generate structured diagnostic information for supported failures. | Must | Controlled failure scenarios produce usable diagnostic records. |
| BR-005 | Psydian shall provide a controlled communication channel for diagnostics. | Must | Structured diagnostic messages can cross the defined kernel/host boundary. |
| BR-006 | A host-side AI Bridge shall receive and process diagnostic information. | Must | Bridge successfully receives and parses a supported diagnostic message. |
| BR-007 | The AI Bridge shall communicate with the configured external AI service. | Must | A valid request can be sent and a response received or a controlled failure reported. |
| BR-008 | The system shall display AI-assisted diagnostic explanations to the user. | Must | A valid AI response is returned to and displayed by the system interface. |
| BR-009 | AI-generated suggestions shall not bypass command validation. | Must | Suggested commands are treated as untrusted input and validated before execution. |
| BR-010 | Privileged or destructive actions shall require user confirmation. | Must | Such actions cannot be executed automatically from AI output. |
| BR-011 | Core shell and kernel functionality shall continue when AI assistance is unavailable. | Must | AI/API/bridge failure does not prevent basic system operation. |
| BR-012 | The project shall support reproducible development and testing through QEMU. | Must | The kernel can be rebuilt and executed in the defined QEMU environment. |
| BR-013 | Optional voice interaction shall use the existing shell/AI processing pipeline. | Could | Voice input is converted to text and processed without creating a separate command architecture. |
| BR-014 | The architecture shall remain extensible for future OS and AI capabilities. | Should | New subsystems can be introduced without redesigning the core kernel/AI boundary. |

## 8. Non-Functional Requirements

- **Reliability:** Core kernel and shell functionality shall not depend on external AI service availability.
- **Security:** The kernel shall remain isolated from direct external AI access.
- **Safety:** AI-generated commands shall be treated as untrusted and shall not automatically perform privileged or destructive operations.
- **Performance:** Basic shell interactions should remain responsive in the QEMU development environment; external AI response time is considered a dependency and shall not block core kernel execution indefinitely.
- **Maintainability:** Kernel functionality shall be organized into clearly defined modules with separated responsibilities.
- **Testability:** Kernel subsystems and host-side AI components shall have defined validation and failure scenarios.
- **Portability:** The MVP shall target x86_64 and QEMU while keeping subsystem boundaries sufficiently modular for future platform work.
- **Fault Isolation:** Failures in AI, voice, or host communication shall not directly destabilize the kernel.
- **Observability:** Kernel events, errors, and diagnostic information shall be recorded through defined logging mechanisms.
- **Reproducibility:** The project shall use version-controlled source, documented toolchain configuration, and a reproducible QEMU environment.
- **Privacy:** Only the diagnostic information necessary for analysis shall be sent to an external AI service.

## 9. Success Metrics

- Successful boot rate of the Psydian kernel in the supported QEMU environment.
- Percentage of defined kernel and shell test cases passing.
- Percentage of controlled diagnostic scenarios producing valid structured records.
- Percentage of supported AI requests successfully completing the kernel/host/AI round trip.
- AI response validation success rate.
- Percentage of AI-generated unsafe or malformed actions correctly rejected by validation.
- Percentage of core system functionality remaining operational during AI-service failure.
- Median response time for local shell commands.
- Median end-to-end AI diagnostic response time.
- Number of critical kernel failures reproducibly diagnosed through the diagnostic pipeline.
- Completion of the planned MVP within the 8-week project period.

## 10. Risks

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| Bare-metal kernel development is more complex than expected | High | High | Follow incremental subsystem development and validate each milestone in QEMU. |
| Rust nightly or bootloader compatibility changes | Medium | High | Pin/document toolchain versions and use current official documentation. |
| Memory-management implementation causes kernel instability | High | High | Implement incrementally and test each memory subsystem independently. |
| Interrupt or exception handling introduces system crashes | High | High | Use controlled test cases and develop handlers incrementally. |
| AI service unavailable or API changes | Medium | Medium | Keep the AI Bridge separate and ensure core OS functionality works without AI. |
| AI produces incorrect or unsafe recommendations | Medium | High | Treat AI output as untrusted, validate responses, and require user confirmation for sensitive actions. |
| Communication-channel failure | Medium | Medium | Use bounded message handling and ensure diagnostic failures do not crash the kernel. |
| 8-week timeline is insufficient for all planned features | High | High | Prioritize MVP features and defer voice, advanced networking, filesystem, GUI, and other stretch goals. |
| Voice integration becomes too complex | Medium | Medium | Implement voice only after the text-based shell/AI pipeline is stable. |
| Hardware-specific behavior differs from QEMU | Medium | Medium | Define QEMU as the primary MVP validation environment and treat physical hardware support as future work. |

## 11. MVP Scope

**In scope:**

- x86_64 bootable kernel.
- Rust bare-metal kernel foundation.
- QEMU-based execution and testing.
- Serial output and kernel logging.
- Required CPU exception and interrupt handling.
- Keyboard input.
- Basic memory-management foundation.
- Interactive shell.
- Structured diagnostics.
- Panic handling and controlled failure testing.
- Host-side AI Bridge.
- Kernel-to-host diagnostic communication.
- External AI API integration.
- AI-assisted diagnostic explanation.
- AI response validation.
- Safe command handling and user confirmation.
- Basic reliability and security validation.
- Technical documentation and reproducible build/run instructions.

**Out of scope for the MVP:**

- Full production-grade operating system.
- Native kernel networking stack.
- Full filesystem implementation.
- Large hardware-driver ecosystem.
- Multi-user operating-system support.
- Autonomous AI control of the kernel.
- Arbitrary automatic execution of AI-generated commands.
- Training an LLM from scratch.
- Production AI infrastructure.
- Full desktop GUI.
- Full voice assistant implementation.
- Primary deployment on physical hardware.
- Enterprise-grade operating-system features.

## 12. Future Scope

- Advanced dynamic voice interaction.
- Local/on-device AI model integration.
- Multiple AI-provider support.
- More advanced diagnostic analysis.
- AI-assisted command completion and system automation.
- Improved shell and terminal interface.
- Filesystem support.
- Networking stack.
- Process scheduling and multitasking.
- Device-driver support.
- Multi-user support and permission management.
- GUI/desktop environment.
- Advanced system monitoring.
- Persistent diagnostic history.
- More sophisticated AI safety and policy enforcement.
- Physical-hardware support.
- UEFI-focused boot support.
