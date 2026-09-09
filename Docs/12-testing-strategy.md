# Testing Strategy

## 1. Testing Objective

The testing strategy for Psydian is designed to verify that the operating-system prototype works correctly at the kernel, subsystem, communication, AI Bridge, and end-to-end levels.

The project follows an incremental testing approach because kernel failures can affect the entire system.

The primary goals are:

- Verify that Psydian boots reliably in QEMU.
- Verify individual kernel subsystems.
- Verify command parsing and execution.
- Verify exception, interrupt, memory, and panic behavior.
- Verify structured diagnostic generation.
- Verify kernel-to-AI-Bridge communication.
- Verify AI request and response processing.
- Verify security and validation boundaries.
- Verify graceful degradation when the AI service is unavailable.
- Measure performance and reliability.
- Provide evidence that the implemented MVP works as specified.

---

# 2. Testing Principles

- Test each subsystem independently before integrating it.
- Prefer deterministic and reproducible test cases.
- Use controlled failure scenarios for kernel diagnostics.
- Keep core kernel tests independent from external AI availability.
- Treat external AI responses as untrusted test inputs.
- Test both successful and failure paths.
- Automate repeatable tests where practical.
- Record test results and known limitations.
- Do not consider a feature complete until its expected error paths have been tested.

---

# 3. Test Levels

Psydian will use the following test levels:

1. Unit Testing
2. Kernel Subsystem Testing
3. Integration Testing
4. Communication/Protocol Testing
5. AI Bridge Testing
6. End-to-End Testing
7. Security Testing
8. Performance Testing
9. Failure and Recovery Testing
10. Manual Demonstration Testing

---

# 4. Unit Testing

Unit tests should verify isolated logic that can be tested without booting the complete kernel.

Potential unit-test targets include:

- Command parser.
- Command validation.
- Argument parsing.
- Diagnostic record creation.
- Message serialization.
- Message deserialization.
- Protocol validation.
- Risk classification.
- AI response parsing.
- AI response schema validation.
- Configuration parsing.
- Utility functions.

Example:

```text
Input:
"meminfo"

Expected:
Command = meminfo
Arguments = []
```

Another example:

```text
Input:
"unknown_command"

Expected:
Validation Error
```

---

# 5. Kernel Subsystem Testing

Kernel subsystems should be tested incrementally as they are implemented.

## 5.1 Boot Testing

Verify:

- Kernel image builds successfully.
- Bootloader successfully loads the kernel.
- Kernel reaches its entry point.
- Kernel initialization does not immediately panic.
- Expected boot logs are produced.

Expected evidence:

```text
Bootloader
    ↓
Kernel Entry
    ↓
[INFO] Psydian kernel booted.
```

---

## 5.2 Serial Testing

Verify:

- UART initializes successfully.
- Single-byte transmission works.
- String transmission works.
- Multiple consecutive log messages work.
- QEMU serial output is visible on the host.
- Serial output remains usable during controlled failures.

---

## 5.3 Logger Testing

Verify:

- `INFO` messages are emitted correctly.
- `WARN` messages are emitted correctly.
- `ERROR` messages are emitted correctly.
- `PANIC` diagnostics are emitted correctly.
- Log formatting remains consistent.
- Shared serial access does not corrupt output.

Example:

```text
[INFO] Kernel initialized
[WARN] Test warning
[ERROR] Test error
```

---

## 5.4 Keyboard Testing

Verify:

- Key press events are received.
- Supported keys are decoded correctly.
- Character input reaches the shell.
- Enter submits a command.
- Backspace behaves correctly.
- Supported modifier keys behave correctly.

---

## 5.5 Shell Testing

Verify:

- Shell starts after kernel initialization.
- Commands are parsed correctly.
- Unknown commands are rejected.
- Invalid arguments are rejected.
- Valid commands execute correctly.
- Command output is displayed.
- Shell remains usable after command errors.

---

## 5.6 Memory Testing

Verify:

- Boot memory information is read correctly.
- Required paging structures are initialized.
- Heap initialization succeeds.
- Valid allocations succeed.
- Deallocations behave correctly where implemented.
- Allocation failures are handled safely.
- Memory-related diagnostics are generated for controlled failures.

---

## 5.7 Exception Testing

Controlled test scenarios should be used for supported CPU exceptions.

Examples:

```text
Divide-by-zero test
Invalid operation test
Page fault test
```

For each supported exception:

```text
Exception
   ↓
Handler
   ↓
Diagnostic
   ↓
Log
   ↓
Controlled Failure / Recovery
```

The test must verify that the expected handler is invoked instead of producing an uncontrolled or unexplained failure.

---

## 5.8 Interrupt Testing

Verify:

- Interrupt descriptor setup.
- Hardware interrupt registration.
- Keyboard interrupt handling.
- Interrupt handler execution.
- End-of-interrupt handling where applicable.
- Normal kernel execution after handling supported interrupts.

---

# 6. Communication Protocol Testing

The communication protocol between the kernel and AI Bridge shall be tested independently of the external AI provider.

## 6.1 Valid Message

Example:

```json
{
  "type": "diagnostic",
  "version": 1,
  "id": "diag-001",
  "payload": {
    "severity": "error",
    "subsystem": "memory",
    "message": "Heap allocation failed"
  }
}
```

Expected result:

```text
Message accepted
→ Parsed
→ Validated
→ Processed
```

## 6.2 Invalid Message

Example:

```json
{
  "type": "unknown",
  "payload": []
}
```

Expected result:

```text
Message rejected
→ Controlled error
→ No execution
```

---

# 7. AI Bridge Testing

The AI Bridge should be testable without depending entirely on a live external AI service.

## 7.1 Request Construction

Verify:

- Correct request type.
- Correct diagnostic context.
- Correct user query handling.
- Input limits.
- Provider-specific request formatting.

## 7.2 Response Parsing

Verify:

- Valid AI response is parsed.
- Missing required fields are detected.
- Incorrect data types are rejected.
- Oversized responses are rejected.
- Unexpected fields are handled safely.

## 7.3 Response Validation

Verify:

- Schema validation.
- Safety validation.
- Suggested-command validation.
- Confirmation requirements.
- Unsupported response handling.

## 7.4 Provider Failure

Test:

- Authentication failure.
- Rate limiting.
- Timeout.
- Network failure.
- Server error.
- Malformed provider response.

Expected result:

```text
Provider Failure
    ↓
AI Bridge
    ↓
Controlled Error
    ↓
Psydian
    ↓
User
```

Core kernel functionality should remain available.

---

# 8. AI / GenAI Evaluation

The GenAI component requires additional evaluation beyond conventional software tests.

The evaluation set should contain controlled diagnostic scenarios with known expected outcomes.

Each scenario may specify:

```text
Input Diagnostic
Expected Facts
Expected Explanation
Acceptable Investigation Steps
Unsupported Claims
Safety Requirements
```

---

# 9. GenAI Evaluation Metrics

## 9.1 Groundedness

Determine whether the AI response stays consistent with the supplied diagnostic information.

## 9.2 Diagnostic Explanation Correctness

Determine whether the AI correctly explains the reported system condition.

## 9.3 Evidence Usage

Determine whether relevant supplied information is used appropriately.

## 9.4 Hallucination Rate

Measure unsupported technical claims.

## 9.5 Suggested-Step Quality

Evaluate whether suggested troubleshooting steps are technically reasonable and relevant.

## 9.6 Safety Compliance

Verify that the AI:

- Does not bypass command validation.
- Does not claim execution authority.
- Does not automatically authorize privileged operations.
- Does not invent system state.

## 9.7 Latency

Measure:

```text
Request Creation
    ↓
Bridge Processing
    ↓
External AI
    ↓
Response Validation
    ↓
Return to Psydian
```

## 9.8 Cost

When a provider is selected, record approximate AI cost per request using the provider's current pricing.

---

# 10. Integration Testing

Integration testing verifies interactions between multiple subsystems.

Important integration paths include:

### Kernel + Serial

```text
Kernel
 ↓
Logger
 ↓
UART
 ↓
QEMU
 ↓
Host Terminal
```

### Kernel + Shell

```text
Keyboard
 ↓
Input Buffer
 ↓
Shell Parser
 ↓
Command Handler
 ↓
Output
```

### Diagnostics + AI Bridge

```text
Kernel Diagnostic
 ↓
Communication Protocol
 ↓
AI Bridge
 ↓
AI Processing
 ↓
Validated Response
```

---

# 11. End-to-End Testing

The complete AI-assisted flow shall be tested as one system.

## 11.1 Diagnostic Flow

```text
System Failure
    ↓
Kernel Exception / Panic
    ↓
Diagnostic Generation
    ↓
Serial / Diagnostic Channel
    ↓
AI Bridge
    ↓
External AI Service
    ↓
AI Response
    ↓
Response Validation
    ↓
Psydian
    ↓
Shell
    ↓
User
```

The test passes when the user receives a relevant, validated explanation without bypassing the system's security controls.

---

# 12. AI Command Suggestion Testing

When the AI returns a command suggestion:

```text
AI Suggestion
    ↓
Bridge
    ↓
Psydian
    ↓
Shell Parser
    ↓
Command Validation
    ↓
Risk Classification
    ↓
Confirmation
    ↓
Execution / Rejection
```

Test cases should include:

- Safe command.
- Invalid command.
- Unknown command.
- Privileged command.
- Destructive command.
- Malformed command.
- Command designed to bypass validation.

Expected behavior is that all suggestions still pass through normal validation.

---

# 13. Failure and Recovery Testing

Psydian shall deliberately test important failure scenarios.

Examples:

- Boot failure.
- Kernel initialization failure.
- Serial initialization failure.
- Invalid command.
- Unsupported command.
- Memory allocation failure.
- Page fault.
- Keyboard input failure.
- Communication failure.
- AI timeout.
- AI authentication failure.
- AI provider outage.
- Malformed AI response.
- Invalid protocol message.

For each scenario:

```text
Failure
  ↓
Detection
  ↓
Controlled Handling
  ↓
Diagnostic
  ↓
Safe State
```

---

# 14. Graceful Degradation Testing

The system must continue basic operation when optional external components fail.

## AI Service Unavailable

Expected:

```text
AI unavailable
    ↓
Controlled error
    ↓
Normal shell remains usable
```

## Voice Unavailable

Expected:

```text
Voice unavailable
    ↓
Keyboard/text interface remains usable
```

## AI Bridge Unavailable

Expected:

```text
AI Bridge unavailable
    ↓
AI assistance unavailable
    ↓
Core kernel/shell remains functional
```

---

# 15. Security Testing

Security testing shall verify the trust boundary.

## 15.1 Input Security

Test:

- Oversized input.
- Malformed messages.
- Invalid command syntax.
- Unexpected message types.
- Unsupported protocol versions.

## 15.2 AI Security

Test:

- Prompt injection text.
- Unsafe AI response.
- Malformed AI output.
- Unsupported claims.
- Arbitrary command suggestions.

## 15.3 Command Security

Test:

- Privileged command.
- Destructive command.
- AI-generated privileged command.
- Confirmation bypass attempts.

## 15.4 Secret Security

Verify that:

- API keys are not committed.
- API keys are not printed.
- API keys are not included in kernel binaries.
- Sensitive request content is not unnecessarily logged.

---

# 16. Performance Testing

Performance testing shall focus on both local kernel behavior and external AI latency.

## 16.1 Kernel Metrics

Measure:

- Boot/initialization time.
- Local command response time.
- Serial logging overhead.
- Keyboard response time.
- Memory allocation performance where measurable.

## 16.2 AI Metrics

Measure:

- AI request latency.
- AI response latency.
- End-to-end diagnostic latency.
- Timeout frequency.
- Retry frequency.

External AI latency shall be measured separately from local kernel processing.

---

# 17. Stress Testing

Where practical, run repeated operations such as:

- Repeated shell commands.
- Repeated keyboard input.
- Repeated diagnostic generation.
- Repeated AI requests.
- Repeated communication messages.
- Repeated allocations.

The goal is to detect:

- Buffer overflows.
- Memory leaks.
- Deadlocks.
- Output corruption.
- State corruption.
- Unbounded resource use.

---

# 18. Regression Testing

Every significant kernel change should be checked against previously validated functionality.

Minimum regression checks:

```text
Boot
 ↓
Serial Output
 ↓
Kernel Initialization
 ↓
Shell
 ↓
Keyboard Input
 ↓
Existing Commands
 ↓
Diagnostics
 ↓
AI Bridge
```

A new subsystem must not silently break an earlier subsystem.

---

# 19. Test Environment

The primary test environment is:

```text
Host Linux
    ↓
Rust nightly toolchain
    ↓
Psydian kernel
    ↓
Bootloader
    ↓
QEMU
```

AI testing additionally requires:

```text
Host Linux
    ↓
AI Bridge
    ↓
Network
    ↓
Configured AI Provider
```

The documented development environment should be used for reproducible testing.

---

# 20. Sample Test Cases

| ID | Scenario | Expected Result | Priority |
|---|---|---|---|
| TEST-001 | Kernel boot | Psydian reaches kernel entry point | High |
| TEST-002 | Serial initialization | Kernel log appears in host terminal | High |
| TEST-003 | Logger INFO | INFO message formatted correctly | High |
| TEST-004 | Unknown shell command | Controlled error displayed | High |
| TEST-005 | Valid shell command | Correct result displayed | High |
| TEST-006 | Keyboard input | Characters reach shell input buffer | High |
| TEST-007 | Invalid command arguments | Validation error displayed | High |
| TEST-008 | Controlled panic | Panic diagnostic is generated | High |
| TEST-009 | Supported exception | Correct handler executes | High |
| TEST-010 | Keyboard interrupt | Input event is processed | High |
| TEST-011 | Heap allocation | Valid allocation succeeds | High |
| TEST-012 | Allocation failure | Controlled diagnostic generated | High |
| TEST-013 | Valid diagnostic message | AI Bridge accepts message | High |
| TEST-014 | Malformed diagnostic | Message rejected | High |
| TEST-015 | AI request | Valid provider request generated | High |
| TEST-016 | Valid AI response | Response parsed and displayed | High |
| TEST-017 | Malformed AI response | Response rejected safely | High |
| TEST-018 | AI timeout | Controlled error; core system remains available | High |
| TEST-019 | AI suggested safe command | Command passes normal validation | High |
| TEST-020 | AI suggested destructive command | Confirmation/validation required | High |
| TEST-021 | Prompt injection input | Treated as untrusted data | High |
| TEST-022 | Missing AI credentials | Controlled configuration error | High |
| TEST-023 | AI Bridge unavailable | Core shell remains usable | High |
| TEST-024 | Voice unavailable | Text interface remains usable | Medium |
| TEST-025 | Repeated AI requests | Resource limits prevent uncontrolled generation | Medium |

---

# 21. Test Evidence

Each completed test should have enough evidence to demonstrate the result.

Possible evidence includes:

- Terminal output.
- QEMU screenshots.
- Kernel logs.
- Test logs.
- Error output.
- AI request/response samples with secrets removed.
- Recorded latency measurements.
- Automated test results.
- Git commit or build identifier.

For demonstration-critical features, screenshots or recordings should be retained where appropriate.

---

# 22. Test Result Format

A simple result format should be used:

```text
Test ID: TEST-001
Scenario: Kernel boot
Status: PASS
Environment: QEMU / x86_64
Expected: Kernel reaches entry point
Actual: Kernel reached entry point
Evidence: Boot log
```

For failures:

```text
Test ID: TEST-018
Scenario: AI timeout
Status: FAIL
Expected: Controlled timeout
Actual: Shell blocked indefinitely
Severity: High
Issue: AI request timeout missing
```

---

# 23. Definition of Done for Testing

A feature is considered test-complete when:

- Expected functionality is implemented.
- Positive cases pass.
- Important failure cases are tested.
- Security implications are reviewed.
- Relevant performance behavior is measured where applicable.
- Integration with dependent components is verified.
- Evidence is recorded.
- Known limitations are documented.

---

# 24. Testing Priority

Given the 8-week project timeline, testing priority shall be:

### Priority 1 — Core System

- Boot.
- Serial.
- Kernel initialization.
- Memory foundation.
- Interrupts/exceptions.
- Keyboard.
- Shell.

### Priority 2 — Diagnostics

- Logging.
- Panic handling.
- Structured diagnostics.

### Priority 3 — AI Integration

- AI Bridge.
- Protocol.
- External AI API.
- Response validation.
- AI-assisted diagnostics.

### Priority 4 — Optional Features

- Advanced shell features.
- Voice integration.
- Additional AI capabilities.

Optional features must not reduce the reliability of the core MVP.

---

# 25. Final End-to-End Validation

Before final demonstration, the complete project should pass the following sequence:

```text
Build
  ↓
Boot in QEMU
  ↓
Kernel Initialization
  ↓
Serial / Logger
  ↓
Keyboard Input
  ↓
Shell
  ↓
Normal Command
  ↓
Controlled Diagnostic Failure
  ↓
Diagnostic Generation
  ↓
AI Bridge
  ↓
External AI
  ↓
Response Validation
  ↓
AI Explanation
  ↓
Display to User
  ↓
AI Failure Test
  ↓
Core Shell Still Works
```

This end-to-end scenario provides the strongest evidence that the Psydian MVP works as an integrated system.
