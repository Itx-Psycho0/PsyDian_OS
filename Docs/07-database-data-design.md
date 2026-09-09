# Database / Data Design

## 1. Purpose

Psydian is a bare-metal operating-system prototype and therefore does not require a conventional application database for its core MVP.

Instead of introducing a database such as MongoDB or PostgreSQL without a concrete requirement, Psydian will use structured in-memory data, kernel data structures, diagnostic records, communication messages, and configuration data appropriate to an operating-system environment.

The data design is therefore divided into:

- Kernel runtime data.
- Shell/input data.
- Diagnostic and logging data.
- Communication protocol data.
- AI request/response data.
- Configuration data.
- Optional persistent data for future versions.

---

# 2. Data Design Principles

- Keep kernel data structures lightweight and deterministic.
- Avoid unnecessary dynamic allocation before the heap is available.
- Keep kernel data separate from host-side AI Bridge data.
- Use fixed-size or bounded structures where appropriate in early kernel stages.
- Treat messages crossing the kernel/host boundary as untrusted input.
- Validate structured data before processing it.
- Keep diagnostic records structured so they can be analyzed by software.
- Avoid storing sensitive information unnecessarily.
- Keep protocol structures versionable for future changes.
- Design data structures around actual subsystem requirements rather than forcing a database model onto the operating system.

---

# 3. Kernel Runtime Data

The kernel will maintain several categories of runtime state.

## 3.1 Boot Information

Boot-time information is received from the bootloader and may include information required to initialize the kernel.

Conceptually:

```text
BootInfo
├── Memory Information
├── Framebuffer Information
├── Bootloader Information
└── Other Platform Information
```

The kernel consumes this information during initialization and converts required values into the internal representations used by its subsystems.

---

## 3.2 Kernel State

The kernel may maintain global state such as:

```text
KernelState
├── Initialization Status
├── Memory State
├── Interrupt State
├── Input State
├── Serial State
├── Diagnostic State
└── Shell State
```

The exact representation will evolve as the individual subsystems are implemented.

---

# 4. Memory Management Data

Memory management will use operating-system-specific data structures rather than database records.

## 4.1 Page Information

The paging subsystem will maintain or access information related to:

- Physical memory frames.
- Virtual memory pages.
- Page-table entries.
- Page mappings.
- Memory regions.
- Allocation state.

Conceptually:

```text
Memory
├── Physical Frames
├── Virtual Pages
├── Page Tables
└── Heap Region
```

## 4.2 Heap State

After the kernel heap is initialized, the allocator will maintain metadata required to determine:

- Free memory.
- Allocated memory.
- Allocation boundaries.
- Deallocation state.

The exact allocator structure will be selected during implementation based on the project requirements and time constraints.

---

# 5. Input Data

The keyboard subsystem will process input events before they are converted into shell commands.

## 5.1 Keyboard Event

A conceptual keyboard event may contain:

```text
KeyboardEvent
├── Key
├── Event Type
└── Modifier State
```

Example:

```text
Key: A
Event: Press
Modifier: Shift
```

The actual representation will depend on the keyboard/input implementation.

## 5.2 Shell Input Buffer

The shell will maintain an input buffer containing the current command being entered.

Conceptually:

```text
InputBuffer
├── Characters
├── Current Length
└── Cursor Position
```

The buffer should have a defined maximum size in early implementation stages to prevent uncontrolled memory use.

---

# 6. Shell Command Data

The shell will convert raw user input into a structured command representation.

## 6.1 Command Structure

Conceptually:

```text
Command
├── Command Name
├── Arguments
└── Execution Context
```

For example:

```text
User Input:
meminfo

Parsed:
Command Name = "meminfo"
Arguments = []
```

Another example:

```text
User Input:
echo hello

Parsed:
Command Name = "echo"
Arguments = ["hello"]
```

## 6.2 Command Registry

The shell may maintain a registry mapping command names to command handlers.

Conceptually:

```text
Command Registry
├── help
├── clear
├── info
├── meminfo
├── panic-test
└── ai
```

This design allows new commands to be added without changing the entire parser.

---

# 7. Logging Data

The logging system will use structured log records.

## 7.1 Log Record

A conceptual record:

```text
LogRecord
├── Level
├── Subsystem
├── Message
└── Optional Context
```

Supported levels:

```text
INFO
WARN
ERROR
PANIC
```

Example:

```json
{
  "level": "ERROR",
  "subsystem": "memory",
  "message": "Heap allocation failed"
}
```

For the earliest kernel implementation, logs may be emitted directly to serial output before more advanced buffering is introduced.

---

# 8. Diagnostic Data

Diagnostics are one of the most important data models in Psydian because they form the bridge between kernel failures and AI-assisted analysis.

## 8.1 Diagnostic Record

A conceptual diagnostic record is:

```json
{
  "type": "diagnostic",
  "version": 1,
  "id": "diag-001",
  "severity": "error",
  "subsystem": "memory",
  "message": "Heap allocation failed",
  "context": {}
}
```

Possible fields include:

| Field | Purpose |
|---|---|
| `type` | Identifies the message as a diagnostic |
| `version` | Allows protocol evolution |
| `id` | Identifies the diagnostic event |
| `severity` | Indicates importance |
| `subsystem` | Identifies the source subsystem |
| `message` | Human-readable explanation |
| `context` | Optional structured diagnostic details |

The final representation may use a binary or compact protocol instead of JSON inside the kernel if memory and performance constraints make that preferable.

---

# 9. Panic Data

A panic is an unrecoverable kernel failure and may produce a specialized diagnostic record.

Conceptually:

```text
PanicRecord
├── Panic Message
├── Source Location
├── Instruction / Execution Context
├── Stack Information
└── Relevant Kernel Context
```

Example conceptual representation:

```json
{
  "type": "panic",
  "version": 1,
  "message": "Kernel panic: test failure",
  "subsystem": "kernel",
  "context": {}
}
```

The amount of context collected will depend on what is safely available at the time of the panic.

---

# 10. Communication Protocol Data

The kernel and AI Bridge need an explicit data format.

## 10.1 Message Types

The MVP may support:

```text
AI_REQUEST
DIAGNOSTIC
AI_RESPONSE
ERROR
STATUS
```

## 10.2 Generic Message Envelope

Conceptually:

```json
{
  "type": "diagnostic",
  "version": 1,
  "id": "msg-001",
  "payload": {}
}
```

The envelope provides a stable outer structure while allowing the payload to vary by message type.

---

# 11. AI Request Data

An AI request may contain the user's question and the relevant system context.

Conceptually:

```json
{
  "type": "ai_request",
  "version": 1,
  "id": "req-001",
  "query": "Why did the kernel panic?",
  "context": {
    "diagnostics": []
  }
}
```

Only context relevant to the request should be included.

The kernel should not transmit arbitrary internal data simply because the AI service can receive it.

---

# 12. AI Response Data

AI responses should be represented in a structured format rather than returning uncontrolled text whenever possible.

Conceptually:

```json
{
  "type": "ai_response",
  "version": 1,
  "id": "resp-001",
  "summary": "The kernel encountered a memory allocation failure.",
  "suggestions": [
    "Inspect allocator state",
    "Check available heap memory"
  ],
  "requires_confirmation": false
}
```

Potential fields include:

- Summary.
- Explanation.
- Suggested actions.
- Suggested commands.
- Confidence or limitations.
- Confirmation requirements.

The final schema will be finalized during AI Bridge implementation.

---

# 13. AI-Suggested Command Data

AI-generated commands are treated as untrusted data.

Conceptually:

```text
AI
 ↓
Suggested Command
 ↓
Shell Parser
 ↓
Command Validation
 ↓
Risk Classification
 ↓
User Confirmation
 ↓
Execution
```

A command suggestion should never be interpreted as an already-authorized kernel operation.

---

# 14. Error Data

The communication layer will use structured errors where practical.

Conceptually:

```json
{
  "type": "error",
  "version": 1,
  "code": "AI_TIMEOUT",
  "message": "AI service request timed out."
}
```

Possible categories include:

```text
INVALID_MESSAGE
INVALID_COMMAND
COMMUNICATION_FAILURE
AI_TIMEOUT
AI_SERVICE_UNAVAILABLE
INVALID_AI_RESPONSE
UNSUPPORTED_OPERATION
```

---

# 15. Configuration Data

Psydian will require configuration values for the host-side AI Bridge.

Examples include:

```text
AI Provider
AI Endpoint
Request Timeout
Protocol Version
Logging Configuration
```

Sensitive values such as API credentials shall not be embedded in kernel binaries or committed to source control.

The host-side configuration should be maintained separately from kernel source.

---

# 16. In-Memory vs Persistent Data

## 16.1 Kernel Runtime

Most kernel state is runtime-only:

```text
Boot Information
Memory State
Interrupt State
Input Buffer
Shell State
Logs
Diagnostic State
```

This data exists while the kernel is running.

## 16.2 Host-side AI Bridge

The AI Bridge may optionally maintain:

```text
Diagnostic History
Request/Response Metadata
Error Logs
Configuration
```

Persistence is optional for the MVP and should only be added if it provides a concrete benefit.

## 16.3 Future Persistent Storage

If Psydian later gains a filesystem, persistent storage could contain:

```text
config/
logs/
diagnostics/
user preferences/
AI interaction metadata/
```

This is future scope and is not required for the initial MVP.

---

# 17. Data Lifecycle

The primary diagnostic data lifecycle is:

```text
Kernel Event
    ↓
Diagnostic Creation
    ↓
Validation
    ↓
Logging
    ↓
Serial / Communication Channel
    ↓
AI Bridge
    ↓
AI Processing
    ↓
AI Response Validation
    ↓
Response Returned
    ↓
Shell Display
```

For a normal command:

```text
Keyboard Input
    ↓
Input Buffer
    ↓
Command Parser
    ↓
Command Validation
    ↓
Command Execution
    ↓
Result
    ↓
Display
```

---

# 18. Data Validation

All data crossing subsystem boundaries shall be validated.

## Kernel Input

Validate:

- Key/input state.
- Command length.
- Command syntax.
- Supported command names.
- Argument structure.

## Communication Messages

Validate:

- Message type.
- Protocol version.
- Message length.
- Required fields.
- Payload structure.

## AI Responses

Validate:

- Response structure.
- Allowed response fields.
- Suggested-command format.
- Safety/confirmation metadata.

Malformed or unexpected messages shall be rejected rather than processed as valid commands or diagnostics.

---

# 19. Data Structures and Algorithms

The central data structures expected in the project include:

### Kernel

- Arrays.
- Fixed-size buffers.
- Ring buffers where required.
- Structs.
- Enums.
- Linked/free lists depending on allocator design.
- Page-table structures.
- Command registries.
- Diagnostic records.

### Shell

- Input buffer.
- Token/argument representation.
- Command registry.
- Command history buffer.

### Communication

- Message envelope.
- Message type enum.
- Bounded receive/transmit buffers.
- Serialization/deserialization structures.

### AI Bridge

- Request/response structures.
- Message queues or bounded buffers where required.
- Diagnostic context structures.
- Validation schemas.

The exact data structures will be selected during implementation based on subsystem constraints.

---

# 20. Data Security

Psydian will follow these data-handling rules:

- Do not transmit unnecessary kernel data to external AI services.
- Do not store API credentials inside the kernel.
- Do not commit secrets to Git.
- Treat received AI content as untrusted.
- Validate all external data before use.
- Avoid exposing sensitive information through ordinary logs.
- Separate diagnostic data from execution authority.
- Keep privileged kernel state inaccessible to the AI Bridge.

---

# 21. Data Design for Scalability

Psydian does not require a traditional database for the MVP.

Scalability should instead be considered at the architectural boundaries:

```text
Kernel
   ↓
Bounded Diagnostic Messages
   ↓
AI Bridge
   ↓
Request Processing
   ↓
External AI Service
```

The AI Bridge can later be extended with:

- Asynchronous message processing.
- Diagnostic queues.
- Persistent diagnostic storage.
- Multiple AI providers.
- Request caching.
- Background processing.

These capabilities should be added only when the project has a concrete requirement for them.

---

# 22. Future Database Consideration

A database may become useful only if future versions introduce features such as:

- Persistent diagnostic history.
- User profiles.
- Long-term AI interaction history.
- System telemetry.
- Multi-user support.
- Remote monitoring.
- Centralized diagnostics from multiple Psydian instances.

For the current MVP, introducing a database would add complexity without solving a core requirement.

Therefore:

> **Psydian MVP does not require a database.**

The primary data model is composed of kernel runtime structures and structured communication records.
