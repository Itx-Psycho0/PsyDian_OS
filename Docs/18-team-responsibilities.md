# Team Responsibilities

## 1. Responsibility Overview

Psydian is a two-student project. Responsibilities are divided according to the major technical areas of the system while keeping the architecture collaborative.

The project uses the following responsibility model:

- **Anurag Singh:** Kernel, systems programming, shell, core OS implementation, debugging, and primary project coordination.
- **Aditya Chauhan:** AI Bridge, AI service integration, prompt engineering, AI response processing, AI testing, and AI/voice-related components.
- **Shared:** Architecture, integration, testing, documentation, version control, final deployment, and project presentation.

Both students should understand the complete system architecture and should be able to explain and defend the project during reviews and demonstration.

---

# 2. Responsibility Matrix

| Anurag Singh | Aditya Chauhan | Mentor Notes |
|---|---|---|
| Project planning and requirement analysis | Project planning and requirement analysis | |
| System architecture and design | Architecture review and implementation support | |
| Bootloader development | AI Bridge development | |
| Kernel initialization | AI service integration | |
| Memory management and paging | AI prompt engineering | |
| Interrupt and exception handling | AI response processing | |
| Heap allocator and kernel logger | Natural-language processing | |
| Command-line shell development | Voice assistant integration (Optional) | |
| Command parser and auto-completion | AI command suggestion module | |
| Kernel debugging and optimization | AI module testing and validation | |
| Documentation website | Testing and bug fixing | |
| GitHub repository and version control | GitHub repository and version control | |
| Project report and presentation | Project report and presentation | |
| Final integration and deployment | Final integration and deployment | |
| Final testing and demonstration | Final testing and demonstration | |

---

# 3. Anurag Singh — Primary Responsibilities

## 3.1 Project Planning and Requirements

Responsibilities:

- Participate in project planning.
- Analyze technical requirements.
- Translate requirements into kernel and system tasks.
- Track implementation milestones.
- Maintain alignment with the 8-week roadmap.
- Identify technical dependencies between kernel components.

## 3.2 System Architecture and Design

Responsibilities:

- Contribute to system architecture.
- Define kernel subsystem boundaries.
- Define interfaces between kernel modules.
- Define the kernel-to-host communication boundary.
- Ensure the AI layer does not directly enter the privileged kernel.

## 3.3 Bootloader Development

Responsibilities:

- Configure the Rust bootloader workflow.
- Generate bootable Psydian images.
- Maintain the x86_64 bare-metal build configuration.
- Validate boot behavior in QEMU.
- Troubleshoot boot and build failures.

## 3.4 Kernel Initialization

Responsibilities:

- Implement kernel entry.
- Establish the initialization sequence.
- Initialize required kernel subsystems.
- Produce boot-time logs.
- Ensure failures during initialization are handled predictably.

Conceptually:

```text
Bootloader
    ↓
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
```

## 3.5 Memory Management and Paging

Responsibilities:

- Process boot-time memory information.
- Implement paging-related foundations.
- Establish required page mappings.
- Implement the kernel heap when required.
- Handle allocation failures.
- Validate memory-related behavior.

## 3.6 Interrupt and Exception Handling

Responsibilities:

- Configure required interrupt structures.
- Implement CPU exception handlers.
- Implement required hardware interrupt handlers.
- Handle keyboard interrupts.
- Generate diagnostics for relevant failures.
- Test controlled exception scenarios.

## 3.7 Heap Allocator and Kernel Logger

Responsibilities:

- Implement kernel heap initialization.
- Select and implement the required allocator strategy.
- Handle allocation/deallocation behavior.
- Detect allocation failures.
- Build the kernel logging abstraction.
- Integrate logger output with serial communication.

## 3.8 Command-Line Shell Development

Responsibilities:

- Implement the shell interface.
- Process keyboard input.
- Manage the command buffer.
- Parse commands.
- Validate commands.
- Dispatch commands.
- Display command results and errors.

## 3.9 Command Parser and Auto-Completion

Responsibilities:

- Implement command tokenization.
- Validate command arguments.
- Handle invalid input.
- Implement command registry behavior.
- Add basic command history where time permits.
- Add basic command auto-completion where time permits.

## 3.10 Kernel Debugging and Optimization

Responsibilities:

- Diagnose kernel build/runtime failures.
- Analyze QEMU serial output.
- Debug memory, interrupt, shell, and boot issues.
- Reduce unnecessary kernel overhead.
- Improve reliability of implemented subsystems.
- Perform regression checks after kernel changes.

## 3.11 Documentation Website

Responsibilities:

- Maintain technical documentation presentation.
- Keep architecture and implementation documentation synchronized with the actual project.
- Publish project progress and major technical milestones where applicable.

---

# 4. Aditya Chauhan — Primary Responsibilities

## 4.1 Project Planning and Requirements

Responsibilities:

- Participate in project planning.
- Review AI-related requirements.
- Identify AI integration dependencies.
- Track AI-related milestones.
- Coordinate AI scope with the 8-week schedule.

## 4.2 Architecture Review and Implementation Support

Responsibilities:

- Review system architecture from the AI integration perspective.
- Verify separation between kernel and host-side AI services.
- Review communication interfaces.
- Support integration between kernel diagnostics and AI processing.
- Identify architectural risks in the AI workflow.

## 4.3 AI Bridge Development

Responsibilities:

- Implement the host-side AI Bridge.
- Receive kernel diagnostic/request messages.
- Parse the communication protocol.
- Validate incoming messages.
- Construct AI requests.
- Process AI responses.
- Return validated results to Psydian.

Core flow:

```text
Psydian
    ↓
Diagnostic / AI Request
    ↓
AI Bridge
    ↓
AI Service
    ↓
Validated Response
    ↓
Psydian
```

## 4.4 AI Service Integration

Responsibilities:

- Configure the selected AI provider.
- Implement provider-specific API communication.
- Handle authentication.
- Handle timeouts.
- Handle provider errors.
- Handle rate limits.
- Keep provider-specific code isolated behind an abstraction.

## 4.5 AI Prompt Engineering

Responsibilities:

- Design system instructions.
- Design diagnostic-analysis prompts.
- Build structured context.
- Reduce unnecessary prompt size.
- Improve evidence grounding.
- Prevent unsupported claims.
- Test different prompt structures.

The prompt should emphasize:

```text
Observed Facts
    ↓
Interpretation
    ↓
Possible Causes
    ↓
Investigation Steps
```

rather than unrestricted autonomous action.

## 4.6 AI Response Processing

Responsibilities:

- Parse AI responses.
- Validate response structure.
- Validate required fields.
- Identify unsupported output.
- Extract explanations.
- Extract troubleshooting steps.
- Extract command suggestions.
- Handle malformed responses safely.

## 4.7 Natural-Language Processing

Responsibilities:

- Process natural-language user requests.
- Map natural-language intent into supported AI requests.
- Normalize relevant text input.
- Prepare context for the AI service.

This functionality should remain within the host-side AI layer and should not bypass the shell's command-validation architecture.

## 4.8 Voice Assistant Integration (Optional)

Responsibilities:

- Research an appropriate speech-to-text approach.
- Integrate speech input if time permits.
- Convert voice into text.
- Send recognized text through the existing shell/AI pipeline.
- Integrate text-to-speech output where practical.
- Ensure keyboard/text interaction remains available as a fallback.

Voice integration shall not create a separate privileged execution path.

## 4.9 AI Command Suggestion Module

Responsibilities:

- Extract command suggestions from AI responses.
- Mark suggestions as untrusted.
- Return suggestions to Psydian.
- Ensure suggested commands pass through normal shell validation.
- Identify commands that require explicit user confirmation.

Flow:

```text
AI Suggestion
    ↓
Psydian
    ↓
Shell Parser
    ↓
Command Validation
    ↓
Risk Check
    ↓
User Confirmation
    ↓
Execution
```

## 4.10 AI Module Testing and Validation

Responsibilities:

- Test AI Bridge behavior.
- Test protocol parsing.
- Test prompt construction.
- Test response validation.
- Test malformed AI responses.
- Test provider failures.
- Evaluate diagnostic correctness.
- Evaluate grounding and hallucination behavior.
- Evaluate AI latency.
- Validate safety controls.

---

# 5. Shared Responsibilities

## 5.1 Architecture

Both students are responsible for understanding:

- Kernel architecture.
- Shell architecture.
- Diagnostic architecture.
- AI Bridge architecture.
- Communication boundaries.
- Security boundaries.
- End-to-end system flow.

Architecture documents should be reviewed jointly.

## 5.2 HLD

**Already completed.**

Use the existing:

```text
Docs/HLD.md
```

rather than creating a second HLD inside this documentation.

## 5.3 LLD

**Already completed.**

Use the existing:

```text
Docs/LLD.md
```

rather than duplicating it.

## 5.4 Testing and Bug Fixing

Both students shall:

- Write and run relevant tests.
- Reproduce reported bugs.
- Fix defects within their responsibility areas.
- Perform regression checks.
- Participate in integration testing.
- Validate the final end-to-end workflow.

## 5.5 GitHub Repository and Version Control

Both students shall:

- Use Git for version control.
- Work through feature branches where appropriate.
- Write meaningful commits.
- Review changes before merging.
- Keep the main branch stable.
- Avoid committing secrets.
- Maintain repository documentation.

Recommended workflow:

```text
feature/*
    ↓
Commit
    ↓
Pull Request
    ↓
Review
    ↓
Tests
    ↓
main
```

## 5.6 Project Report and Presentation

Both students shall:

- Contribute to final documentation.
- Prepare architecture diagrams.
- Prepare implementation evidence.
- Prepare demonstration scenarios.
- Understand major design decisions.
- Be able to explain trade-offs.
- Participate in mentor reviews.
- Participate in the final presentation.

## 5.7 Final Integration and Deployment

Both students shall participate in:

- Kernel integration.
- AI Bridge integration.
- Communication testing.
- Boot-image generation.
- QEMU testing.
- Configuration validation.
- Demo-environment preparation.

## 5.8 Final Testing and Demonstration

Both students shall verify:

```text
Boot
 ↓
Kernel
 ↓
Shell
 ↓
Diagnostic
 ↓
AI Bridge
 ↓
AI Service
 ↓
Validated Response
 ↓
Shell
 ↓
User
```

Both students should be capable of demonstrating the system without relying exclusively on the other team member.

---

# 6. Responsibility Boundaries

The project should maintain clear ownership while avoiding strict silos.

```text
                    Psydian
                       │
        ┌──────────────┴──────────────┐
        │                             │
   Kernel / Systems               AI / Bridge
   Anurag Singh                   Aditya Chauhan
        │                             │
        └──────────────┬──────────────┘
                       │
                 Shared Integration
```

### Kernel Side

Primary owner:

```text
Anurag Singh
```

Core areas:

```text
Boot
Memory
Interrupts
Input
Shell
Logger
Kernel Diagnostics
```

### AI Side

Primary owner:

```text
Aditya Chauhan
```

Core areas:

```text
AI Bridge
AI Provider
Prompt Engineering
Response Processing
AI Evaluation
Voice
```

### Shared Boundary

Both:

```text
Protocol
Security
Testing
Integration
GitHub
Documentation
Deployment
Presentation
```

---

# 7. Collaboration Rules

## Rule 1 — No Black Boxes

Each student must understand the component they integrate with.

For example:

- Kernel developer should understand how the AI Bridge receives diagnostics.
- AI developer should understand what diagnostic data the kernel produces.

## Rule 2 — Stable Interfaces

Changes to shared interfaces should be communicated before implementation.

Important shared interfaces include:

```text
Diagnostic Message
Communication Protocol
AI Response Structure
Shell Command Interface
```

## Rule 3 — Shared Debugging

A failure crossing the ownership boundary should be investigated jointly.

Example:

```text
Kernel sends diagnostic
        ↓
AI Bridge fails
```

The kernel owner and AI owner should inspect the complete path rather than assuming the other component is responsible.

## Rule 4 — Shared Final Knowledge

Both students should understand:

- Why Rust is used.
- Why QEMU is used.
- Why the kernel uses `no_std`.
- Why networking is outside the kernel MVP.
- Why the AI Bridge exists.
- How diagnostics reach the AI service.
- How AI output is validated.
- Why AI cannot directly execute privileged commands.

---

# 8. Mentor Notes

This column is intentionally left blank for mentor comments, review notes, corrections, approvals, or responsibility adjustments.

| Anurag Singh | Aditya Chauhan | Mentor Notes |
|---|---|---|
|  |  |  |
|  |  |  |
|  |  |  |
|  |  |  |

---

# 9. Responsibility Review

Responsibilities should be reviewed at major project checkpoints:

```text
End of Week 2
    ↓
Review Kernel / AI Progress

End of Week 4
    ↓
Review Shell / AI Bridge Readiness

End of Week 6
    ↓
Review Integration

End of Week 8
    ↓
Final Responsibility Review
```

The responsibility allocation may be adjusted if implementation priorities change, but changes should be documented.

---

# 10. Responsibility Success Criteria

The team responsibility plan is successful when:

- Both students contribute meaningful implementation work.
- Each major component has a clear primary owner.
- Shared components are reviewed jointly.
- Neither student becomes a single point of failure for final demonstration.
- Both students understand the complete architecture.
- Both students can explain their technical decisions during evaluation.
- Work is visible through Git history and documented milestones.

---

# 11. Final Responsibility Summary

| Anurag Singh | Aditya Chauhan | Mentor Notes |
|---|---|---|
| Kernel and OS foundation | AI and GenAI foundation | |
| Bootloader and QEMU integration | AI Bridge | |
| Memory, paging, interrupts, exceptions | AI service/API integration | |
| Keyboard and shell | Prompt engineering | |
| Command parser and auto-completion | AI response processing | |
| Heap allocator and logger | Natural-language processing | |
| Kernel diagnostics and debugging | AI command suggestions | |
| Kernel optimization | AI testing and validation | |
| Documentation website | Optional voice assistant | |
| Shared testing and bug fixing | Shared testing and bug fixing | |
| Shared GitHub/version control | Shared GitHub/version control | |
| Shared documentation/report | Shared documentation/report | |
| Shared integration/deployment | Shared integration/deployment | |
| Shared final testing/demo | Shared final testing/demo | |

