# Observability

## 1. Purpose

Observability in Psydian is the ability to understand the internal state and behavior of the operating-system prototype through logs, diagnostics, metrics, and traceable events.

Because Psydian contains a bare-metal kernel, a host-side AI Bridge, and an external AI service, observability must cover all three layers while keeping their boundaries clear.

The primary goals are:

- Understand kernel boot and initialization behavior.
- Detect and diagnose kernel failures.
- Track shell and command-processing events.
- Observe diagnostic generation.
- Track kernel-to-host communication.
- Monitor AI Bridge processing.
- Measure external AI latency and failures.
- Correlate a diagnostic request with its resulting AI response.
- Distinguish kernel failures from host/AI failures.
- Provide useful evidence during development, testing, and demonstration.

---

# 2. Observability Architecture

```text
                    Psydian Kernel
                         │
          ┌──────────────┼──────────────┐
          │              │              │
       Kernel Logs    Diagnostics     Events
          │              │              │
          └──────────────┼──────────────┘
                         ↓
              Serial / Diagnostic Channel
                         ↓
                  Host AI Bridge
                         │
          ┌──────────────┼──────────────┐
          │              │              │
       Bridge Logs    Metrics         Errors
          │              │              │
          └──────────────┼──────────────┘
                         ↓
                 External AI Service
                         │
              Provider/API Response
                         ↓
                 AI Bridge Logs
                         ↓
                  Psydian Response
```

The architecture deliberately keeps observability separate from command authority.

---

# 3. Observability Layers

Psydian observability is divided into:

```text
Layer 1 → Kernel Observability
Layer 2 → Communication Observability
Layer 3 → AI Bridge Observability
Layer 4 → External AI Dependency Observability
Layer 5 → End-to-End Observability
```

---

# 4. Kernel Observability

The kernel shall provide low-level logging through the available serial output mechanism.

Examples:

```text
[INFO] Psydian kernel booted.
[INFO] Serial initialized.
[INFO] Memory initialized.
[INFO] Interrupt subsystem initialized.
[INFO] Keyboard initialized.
[INFO] Shell initialized.
```

Errors:

```text
[ERROR] Heap allocation failed.
[ERROR] Keyboard initialization failed.
```

Warnings:

```text
[WARN] Memory region unavailable.
```

Panic:

```text
[PANIC] Kernel encountered an unrecoverable error.
```

Kernel logs are primarily intended for:

- Development.
- Debugging.
- Testing.
- Panic analysis.
- System diagnostics.

---

# 5. Log Levels

Psydian shall use consistent log levels.

## INFO

Normal system events.

Example:

```text
[INFO] Kernel initialized.
```

## WARN

Unexpected but non-fatal conditions.

Example:

```text
[WARN] Optional feature unavailable.
```

## ERROR

A failure that affects a component but may not stop the whole system.

Example:

```text
[ERROR] AI Bridge communication failed.
```

## PANIC

An unrecoverable kernel condition.

Example:

```text
[PANIC] Page fault during kernel execution.
```

---

# 6. Structured Log Format

Logs should remain consistent and machine-readable where practical.

Conceptual structure:

```text
[TIMESTAMP][LEVEL][SUBSYSTEM][EVENT_ID] MESSAGE
```

Example:

```text
[INFO][BOOT][BOOT-001] Psydian kernel booted.
```

Another example:

```text
[ERROR][MEMORY][MEM-003] Heap allocation failed.
```

During early development, timestamps may be omitted if reliable timekeeping has not yet been implemented.

The project should not require a full timestamp subsystem merely for logging.

---

# 7. Event Categories

Important observability events include:

### Boot Events

```text
KERNEL_BOOT
BOOT_COMPLETE
SUBSYSTEM_INIT
BOOT_FAILURE
```

### Memory Events

```text
MEMORY_INIT
PAGE_TABLE_INIT
HEAP_INIT
ALLOC_SUCCESS
ALLOC_FAILURE
```

### Interrupt Events

```text
EXCEPTION
HARDWARE_INTERRUPT
HANDLER_FAILURE
```

### Shell Events

```text
COMMAND_RECEIVED
COMMAND_VALIDATED
COMMAND_EXECUTED
COMMAND_FAILED
```

### Diagnostic Events

```text
DIAGNOSTIC_CREATED
DIAGNOSTIC_SENT
DIAGNOSTIC_RECEIVED
```

### AI Events

```text
AI_REQUEST_STARTED
AI_REQUEST_SENT
AI_RESPONSE_RECEIVED
AI_RESPONSE_VALIDATED
AI_REQUEST_FAILED
```

---

# 8. Diagnostic Observability

Diagnostics should contain enough contextual information to identify:

- What happened.
- Which subsystem was affected.
- Severity.
- Relevant message.
- Correlation/request identifier where available.
- Additional context when safely available.

Conceptual structure:

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

# 9. Request IDs and Correlation IDs

Psydian should use identifiers to correlate related events.

Example:

```text
Diagnostic ID:
diag-001
```

The corresponding AI request may use:

```text
Request ID:
req-001
```

Conceptual relationship:

```text
diag-001
   ↓
req-001
   ↓
AI Provider Request
   ↓
req-001
   ↓
AI Response
```

This makes it easier to determine which AI response corresponds to which diagnostic.

---

# 10. AI Bridge Logging

The AI Bridge should log operational metadata such as:

```text
Request ID
Message Type
Processing State
Provider
Model
Response Status
Validation Result
Latency
Error Code
```

Example:

```text
[INFO] req-001 DIAGNOSTIC_RECEIVED
[INFO] req-001 AI_REQUEST_STARTED
[INFO] req-001 AI_RESPONSE_RECEIVED latency=1840ms
[INFO] req-001 RESPONSE_VALIDATED
```

Sensitive request payloads should not be logged by default.

---

# 11. Error Codes

Stable internal error codes should be used.

Examples:

```text
COMM-001  Communication failure
COMM-002  Invalid message
AI-001    AI service unavailable
AI-002    AI timeout
AI-003    Invalid AI response
AI-004    Authentication failure
CMD-001   Invalid command
MEM-001   Memory initialization failure
MEM-003   Heap allocation failure
PANIC-001 Kernel panic
```

The exact code registry will be finalized as the implementation grows.

---

# 12. Metrics

Metrics should focus on measurable system behavior rather than collecting unnecessary information.

## Kernel Metrics

Potential metrics:

- Boot success/failure count.
- Kernel initialization duration.
- Exception count.
- Panic count.
- Heap allocation failures.
- Shell command processing time.
- Keyboard input processing rate.

## Communication Metrics

Potential metrics:

- Messages sent.
- Messages received.
- Invalid messages.
- Communication failures.
- Average message processing time.
- Communication latency.

## AI Bridge Metrics

Potential metrics:

- AI requests.
- Successful AI requests.
- Failed AI requests.
- AI timeout count.
- Response validation failures.
- AI request latency.
- AI response latency.
- Retry count.
- Provider error rate.

## AI Cost Metrics

When supported by the selected provider:

- Input token usage.
- Output token usage.
- Estimated request cost.

---

# 13. Recommended Performance Metrics

The MVP should prioritize:

```text
Boot Time
Local Command Latency
Diagnostic Processing Latency
Communication Latency
AI Request Latency
AI Response Latency
End-to-End Diagnostic Latency
```

The end-to-end AI latency can be represented as:

```text
Kernel
  ↓
Diagnostic Creation
  ↓
Communication
  ↓
AI Bridge
  ↓
External AI
  ↓
Response Validation
  ↓
Communication
  ↓
Psydian
```

---

# 14. AI Latency Breakdown

The AI Bridge should ideally measure separate stages:

```text
Request Queue Time
        +
Prompt Construction Time
        +
Network/API Time
        +
Response Parsing Time
        +
Validation Time
        =
AI Processing Time
```

This allows the team to identify whether delays are caused by:

- Local processing.
- Network.
- Provider latency.
- Validation.

---

# 15. Tracing

The complete AI-assisted diagnostic path should be traceable.

Conceptual trace:

```text
User Request
   ↓
Psydian Shell
   ↓
Kernel Diagnostic
   ↓
Diagnostic Message
   ↓
AI Bridge
   ↓
AI Provider
   ↓
AI Response
   ↓
Response Validation
   ↓
Psydian Shell
   ↓
User
```

A request identifier should be retained throughout the flow where practical.

---

# 16. Kernel-to-Host Trace

For a diagnostic event:

```text
Kernel Event
    ↓
Diagnostic ID
    ↓
Serial Transmission
    ↓
Bridge Receive
    ↓
Bridge Processing
```

This helps determine whether a problem occurred in:

```text
Kernel
or
Communication
or
AI Bridge
```

---

# 17. External AI Trace

The AI Bridge should distinguish between:

```text
Request Created
Request Sent
Provider Response Received
Response Parsed
Response Validated
Response Returned
```

This provides enough information to diagnose provider/API failures without logging sensitive content.

---

# 18. Error Observability

Errors shall receive stable internal codes and, where applicable, request/diagnostic identifiers.

User-facing error messages should remain safe and understandable.

Do not expose:

- API credentials.
- Internal secrets.
- Unnecessary stack traces.
- Sensitive diagnostic information.

Example:

```text
[ERROR] AI assistance unavailable.
Request: req-001
Reason: AI request timeout.
```

---

# 19. Panic Observability

Kernel panics are particularly important because the kernel may no longer be able to perform normal processing.

The panic path should attempt to emit:

```text
[PANIC]
Subsystem
Message
Relevant context
Diagnostic ID
```

Example:

```text
[PANIC][MEMORY][PANIC-001]
Heap allocator entered unrecoverable state.
```

The panic handler should avoid depending on complex functionality that may itself be unavailable during a severe failure.

---

# 20. Failure Classification

Observability should make it possible to distinguish:

```text
Kernel Failure
      ↓
Kernel diagnostic

Communication Failure
      ↓
Protocol / transport error

AI Bridge Failure
      ↓
Bridge error

External AI Failure
      ↓
Provider/API error
```

This distinction is essential for debugging the complete system.

---

# 21. Alerting

Psydian is an academic prototype rather than a production cloud service, so a full enterprise alerting system is not required.

However, important failure conditions should be clearly surfaced.

Potential alert conditions:

- Repeated kernel initialization failures.
- Repeated communication failures.
- AI provider unavailable.
- High AI timeout rate.
- Repeated malformed AI responses.
- Unexpected increase in panic events.
- Repeated allocation failures.
- Repeated protocol validation failures.

In the MVP, these can be represented through terminal/serial logs rather than a separate alert-management service.

---

# 22. Observability During Development

During development, the primary observability interface will be:

```text
Psydian
  ↓
Serial Output
  ↓
QEMU
  ↓
Host Terminal
```

Example:

```text
[INFO][BOOT] Psydian kernel booted.
[INFO][SERIAL] UART initialized.
[INFO][MEMORY] Memory subsystem initialized.
[INFO][INT] Interrupt subsystem initialized.
[INFO][INPUT] Keyboard initialized.
[INFO][SHELL] Shell initialized.
```

This provides immediate feedback during subsystem development.

---

# 23. Observability During AI Diagnostics

An AI-assisted diagnostic session may produce:

```text
[ERROR][MEMORY][MEM-003]
Heap allocation failed.

[INFO][AI][req-001]
Diagnostic sent to AI Bridge.

[INFO][AI][req-001]
AI analysis received.

[INFO][AI][req-001]
Response validated.
```

The shell can then display:

```text
AI Analysis:
The heap allocation request could not be satisfied.

Suggested Investigation:
1. Inspect allocator state.
2. Check available heap memory.
```

---

# 24. Privacy and Logging Rules

Observability must not become a source of unnecessary data exposure.

Do not log:

```text
API keys
Authentication tokens
Secrets
Unnecessary personal data
Unnecessary complete AI payloads
```

Where sensitive information may appear in user input or diagnostics, the bridge should filter or avoid persistent logging.

---

# 25. Observability Data Retention

For the MVP:

- Kernel logs may primarily remain in serial output.
- AI Bridge logs may be stored locally during development.
- Persistent long-term telemetry is not required.
- Diagnostic history storage is optional.

Future versions may add persistent diagnostic storage when a filesystem is available.

---

# 26. AI Observability

AI-specific metadata should include:

```text
Request ID
Provider
Model
Latency
Status
Validation Result
Error Code
Token Usage (if available)
```

The project should record model/provider information so AI behavior can be compared between versions.

---

# 27. Observability for AI Evaluation

Observability data can support GenAI evaluation.

For example:

```text
Request ID
    ↓
Diagnostic Input
    ↓
Model
    ↓
Latency
    ↓
Response
    ↓
Validation Result
    ↓
Human Evaluation
```

This allows the team to associate model behavior with a specific test scenario.

---

# 28. Development Dashboard / Visualization

A dedicated dashboard is not required for the MVP.

For the initial implementation, structured serial/terminal output is sufficient.

A future monitoring interface could display:

```text
Kernel Status
Memory Usage
Exception Count
Panic Count
Shell Commands
AI Requests
AI Latency
AI Errors
Diagnostic History
```

This is future scope.

---

# 29. Observability Test Cases

| ID | Scenario | Expected Result | Priority |
|---|---|---|---|
| OBS-001 | Kernel boot | Boot event logged | High |
| OBS-002 | Serial initialization | Serial status logged | High |
| OBS-003 | Memory initialization | Memory event logged | High |
| OBS-004 | Shell command | Command event logged | High |
| OBS-005 | Kernel error | ERROR log generated | High |
| OBS-006 | Kernel panic | PANIC diagnostic generated | High |
| OBS-007 | Diagnostic sent | Communication event logged | High |
| OBS-008 | AI request | Request ID recorded | High |
| OBS-009 | AI response | Response validation event recorded | High |
| OBS-010 | AI timeout | AI timeout error recorded | High |
| OBS-011 | Invalid protocol message | Validation failure recorded | High |
| OBS-012 | Provider failure | External-service error distinguished from kernel failure | High |
| OBS-013 | Repeated failures | Repeated failure is visible in logs/metrics | Medium |
| OBS-014 | Secret exposure test | Sensitive credentials absent from logs | High |

---

# 30. Observability Requirements

| ID | Requirement | Priority |
|---|---|---|
| OBS-REQ-001 | Kernel shall provide structured log levels. | Must |
| OBS-REQ-002 | Kernel errors shall produce diagnostic information. | Must |
| OBS-REQ-003 | Panic paths shall attempt to emit diagnostics. | Must |
| OBS-REQ-004 | Communication events shall be traceable. | Must |
| OBS-REQ-005 | AI requests shall have identifiers. | Should |
| OBS-REQ-006 | AI latency shall be measurable. | Should |
| OBS-REQ-007 | AI errors shall be distinguishable from kernel errors. | Must |
| OBS-REQ-008 | Sensitive credentials shall not be logged. | Must |
| OBS-REQ-009 | Diagnostic records shall contain subsystem and severity information. | Must |
| OBS-REQ-010 | Core observability shall function without the external AI service. | Must |
| OBS-REQ-011 | External provider/model information should be recorded for AI evaluation. | Should |
| OBS-REQ-012 | Observability data should support debugging of end-to-end AI diagnostics. | Must |

---

# 31. MVP Observability Scope

## Required

- Kernel serial logging.
- INFO/WARN/ERROR/PANIC log levels.
- Boot and initialization logs.
- Exception/panic diagnostics.
- Shell operation logs where useful.
- Structured diagnostic records.
- Communication error logging.
- AI Bridge operational logging.
- Request/diagnostic identifiers where practical.
- AI latency measurement.
- AI failure reporting.
- Secret-safe logging.
- End-to-end diagnostic traceability.

## Optional

- Persistent diagnostic history.
- Metrics dashboard.
- Distributed tracing.
- Advanced telemetry storage.
- Automated alerting infrastructure.
- Remote monitoring.
- Log aggregation service.
- OpenTelemetry integration.

Optional observability infrastructure must not delay core Psydian development.

---

# 32. Final Observability Flow

```text
Kernel Event
      ↓
Structured Log / Diagnostic
      ↓
Serial Channel
      ↓
Host AI Bridge
      ↓
Request ID / Processing State
      ↓
External AI Service
      ↓
Response / Latency / Status
      ↓
Response Validation
      ↓
Diagnostic Result
      ↓
Psydian Shell
      ↓
User
```

---

# 33. Observability Principles

### Visibility

Important system behavior should produce observable evidence.

### Correlation

Related events should be traceable through identifiers.

### Separation

Kernel, communication, bridge, and external AI failures should be distinguishable.

### Security

Observability must not expose credentials or unnecessary sensitive information.

### Low Overhead

Logging and measurement should not excessively interfere with kernel operation.

### Determinism

Development and testing output should remain predictable and reproducible.

### AI Transparency

AI-generated information should remain distinguishable from original kernel evidence.

### Graceful Degradation

Core observability should continue even when the AI service is unavailable.
