# Requirements Traceability Matrix

## 1. Purpose

The Requirements Traceability Matrix (RTM) connects Psydian's requirements with its design, implementation, testing, and final validation.

The purpose of the RTM is to ensure that:

```text
Requirement
    ↓
Design
    ↓
Implementation
    ↓
Test
    ↓
Validation
```

Every important requirement should have a clear path to implementation and evidence.

The RTM also helps identify:

- Requirements that are not implemented.
- Features that have not been tested.
- Tests that do not map to a requirement.
- Documentation/design elements that have no corresponding requirement.
- Scope that should be deferred instead of silently becoming mandatory.

---

# 2. Traceability Levels

Psydian uses the following traceability chain:

```text
Business Requirement
        ↓
Product Requirement
        ↓
Technical Requirement
        ↓
Architecture / Design
        ↓
Implementation Module
        ↓
Test Case
        ↓
Evidence
```

For project practicality, requirements can be traced using the identifiers defined in the BRD, TRD, security, testing, and other project documents.

---

# 3. Requirement ID Conventions

Recommended identifiers:

| Prefix | Meaning |
|---|---|
| BR | Business Requirement |
| PR | Product Requirement |
| UX | User Experience Requirement |
| TR | Technical Requirement |
| SEC | Security Requirement/Test |
| API | API Requirement |
| DEP | Deployment Requirement |
| COST | Cost Requirement |
| OBS | Observability Requirement |
| TEST | Test Case |
| ADR | Architecture Decision Record |

The exact identifiers may evolve as the documentation is consolidated.

---

# 4. Core Requirement Traceability

| Requirement ID | Requirement | Design / Component | Implementation Area | Test / Validation | Status |
|---|---|---|---|---|---|
| BR-001 | Psydian shall boot as an x86_64 kernel in QEMU. | Boot Architecture | `kernel/boot`, kernel entry | TEST-001 | Planned / In Progress |
| BR-002 | Users shall interact through a command-line shell. | Shell Architecture | `kernel/shell`, `kernel/input` | TEST-005, TEST-006 | Planned / In Progress |
| BR-003 | Psydian shall provide kernel logging. | Observability Design | `kernel/logger`, `kernel/serial` | TEST-002, TEST-003 | Planned / In Progress |
| BR-004 | Psydian shall generate structured diagnostics. | Diagnostic Architecture | `kernel/diagnostics` | TEST-008, TEST-009, TEST-012 | Planned |
| BR-005 | Psydian shall provide a controlled communication channel. | Communication Protocol | `kernel/serial`, protocol layer | TEST-013, TEST-014 | Planned |
| BR-006 | Host-side AI Bridge shall process diagnostics. | GenAI Architecture | `ai-bridge/protocol`, `ai-bridge/diagnostics` | TEST-013, TEST-015 | Planned |
| BR-007 | AI Bridge shall communicate with external AI service. | API Specification | `ai-bridge/provider` | TEST-015, integration test | Planned |
| BR-008 | AI-assisted explanations shall be returned to the user. | GenAI Architecture | `ai-bridge/context`, `ai-bridge/prompt`, shell output | TEST-016 | Planned |
| BR-009 | AI suggestions shall not bypass validation. | Security Design | `shell/validation`, `ai-bridge/safety` | TEST-019, TEST-020 | Planned |
| BR-010 | Privileged/destructive actions require confirmation. | Security Architecture | shell authorization/risk layer | TEST-020 | Planned |
| BR-011 | Core functionality shall continue during AI failure. | Reliability Architecture | AI Bridge error path | TEST-018, TEST-023 | Planned |
| BR-012 | Builds shall be reproducible in QEMU environment. | Deployment / CI/CD | Cargo/toolchain/QEMU scripts | CI + TEST-001 | In Progress |
| BR-013 | Optional voice shall reuse existing pipeline. | GenAI / Voice Architecture | Voice layer | Voice integration test | Deferred / Optional |
| BR-014 | Architecture shall support future extensions. | HLD / LLD | Module boundaries | Architecture review | Ongoing |

---

# 5. Technical Requirement Traceability

## Kernel

| TR ID | Technical Requirement | Component | Test | Evidence |
|---|---|---|---|---|
| TR-001 | Boot x86_64 kernel in QEMU | Bootloader + Kernel | TEST-001 | QEMU boot log |
| TR-002 | Compile for `x86_64-unknown-none` | Cargo / Kernel | Kernel build | Successful build |
| TR-003 | Operate without `std` | Kernel | Build validation | Successful `no_std` build |
| TR-004 | Provide defined kernel entry point | `kernel/main.rs` | TEST-001 | Kernel reaches entry |
| TR-005 | Provide serial output | Serial module | TEST-002 | Serial log |
| TR-006 | Provide structured logging | Logger module | TEST-003 | Log output |
| TR-007 | Support CPU exception handling | Interrupt module | TEST-009 | Exception output |
| TR-008 | Support required hardware interrupts | Interrupt module | TEST-010 | Interrupt evidence |
| TR-009 | Process keyboard input | Input module | TEST-006 | Typed input visible |
| TR-010 | Provide MVP memory management | Memory module | TEST-011, TEST-012 | Allocation/memory evidence |
| TR-011 | Provide controlled panic handler | Panic module | TEST-008 | Panic diagnostic |
| TR-012 | Generate structured diagnostics | Diagnostics module | TEST-008, TEST-012 | Diagnostic record |
| TR-013 | Parse and validate commands | Shell module | TEST-004, TEST-005, TEST-007 | Command results |
| TR-014 | Display results and errors | Shell | TEST-005, TEST-007 | Shell output |
| TR-015 | Support command history | Shell | Shell test | History behavior |
| TR-016 | Support auto-completion | Shell | Shell test | Completion behavior |
| TR-017 | Expose controlled host communication | Protocol / Serial | TEST-013, TEST-014 | Protocol logs |
| TR-018 | AI Bridge parses Psydian messages | AI Bridge Protocol | TEST-013 | Bridge test output |
| TR-019 | Construct AI requests | Context / Prompt | TEST-015 | Request fixture |
| TR-020 | Use HTTPS/REST for AI provider | Provider module | Integration test | Provider response |
| TR-021 | Validate AI responses | Validation module | TEST-016, TEST-017 | Validation result |
| TR-022 | AI commands pass through validation | Shell + Safety | TEST-019, TEST-020 | Execution/ rejection evidence |
| TR-023 | Require confirmation for dangerous actions | Security layer | TEST-020 | Confirmation evidence |
| TR-024 | AI failure does not terminate kernel | Error handling | TEST-018, TEST-023 | Shell remains usable |
| TR-025 | Communication failures are controlled | Protocol | TEST-014 | Controlled error |
| TR-026 | Diagnostic messages use defined structure | Diagnostics / Protocol | TEST-013 | Valid message fixture |
| TR-027 | Configuration outside kernel source | Config | Security review | Repository scan |
| TR-028 | Credentials not committed | Security | Secret scan | CI security result |
| TR-029 | Reproducible QEMU build/run | CI/CD / Deployment | Build + smoke test | CI artifacts |
| TR-030 | Optional voice reuses existing pipeline | Voice module | Voice integration test | Optional |

---

# 6. API Traceability

| API ID | Requirement | Design | Implementation | Test |
|---|---|---|---|---|
| API-001 | Construct validated AI requests | AI Bridge API | Request builder | API request test |
| API-002 | Use HTTPS | External Provider Interface | Provider client | Live/integration test |
| API-003 | Protect credentials | Security Design | Host configuration | Secret scan |
| API-004 | Validate external responses | Validation layer | Response validator | AI response tests |
| API-005 | Handle timeout/transient failures | Reliability design | Provider client | TEST-018 |
| API-006 | AI commands cannot bypass shell | Security flow | Shell validation | TEST-019, TEST-020 |
| API-007 | Requests have traceable IDs | Observability | Request ID generation | Observability test |
| API-008 | Internal messages have versions | Protocol | Message envelope | Protocol test |
| API-009 | Provider-specific code is isolated | Provider abstraction | Provider adapter | Architecture/code review |
| API-010 | Core shell continues during AI failure | Reliability | Error handling | TEST-023 |
| API-011 | Request/response limits | Validation | Size checks | Boundary tests |
| API-012 | Sensitive request content is not unnecessarily logged | Security | Logging policy | Security test |
| API-013 | Retry behavior is bounded | Reliability | Retry policy | Failure/retry test |
| API-014 | Optional health/status support | Observability | Bridge status | Optional |
| API-015 | Provider replacement is possible | Provider abstraction | Adapter interface | Architecture review |

---

# 7. Security Traceability

| Security ID | Security Requirement | Design Element | Test |
|---|---|---|---|
| SEC-001 | Invalid shell commands are rejected | Shell validation | TEST-004 |
| SEC-002 | Oversized input is controlled | Input limits | Boundary test |
| SEC-003 | Malformed protocol messages are rejected | Protocol validation | TEST-014 |
| SEC-004 | Unsupported protocol versions are rejected | Version validation | Protocol test |
| SEC-005 | Malformed AI responses are rejected | Response validation | TEST-017 |
| SEC-006 | Privileged AI suggestions require confirmation | Risk/authorization layer | TEST-020 |
| SEC-007 | Destructive AI suggestions are controlled | Safety layer | TEST-020 |
| SEC-008 | Prompt injection is treated as untrusted data | Prompt/security boundary | Prompt injection test |
| SEC-009 | Missing credentials produce controlled error | Config/error handling | Provider config test |
| SEC-010 | AI timeout is controlled | Retry/timeout layer | TEST-018 |
| SEC-011 | Secrets are absent from logs | Logging policy | Secret scanning |
| SEC-012 | Rate/resource control prevents uncontrolled generation | AI Bridge limits | Stress test |
| SEC-013 | AI Bridge failure does not break kernel | Fault isolation | TEST-023 |
| SEC-014 | Unexpected external response is rejected | Validation | TEST-017 |

---

# 8. Deployment Traceability

| DEP ID | Requirement | Implementation | Validation |
|---|---|---|---|
| DEP-001 | Clean repository can build project | Toolchain + Cargo | Clean build |
| DEP-002 | Build bootable BIOS image | Bootloader/build.rs | Image generation |
| DEP-003 | Image runs in QEMU | QEMU scripts | QEMU smoke test |
| DEP-004 | Serial output available on host | UART/QEMU | Serial test |
| DEP-005 | AI Bridge runs separately | `ai-bridge/` | Bridge startup test |
| DEP-006 | Bridge uses HTTPS/REST | Provider module | API integration test |
| DEP-007 | Credentials remain outside kernel/repository | Config/security | Secret scan |
| DEP-008 | AI failure does not break core kernel | Reliability layer | Failure test |
| DEP-009 | Stable builds have identifiable commits/tags | Git | Release review |
| DEP-010 | Useful artifacts retained | CI/CD | Artifact inspection |
| DEP-011 | Known-good builds can be restored | Git tags/artifacts | Rollback test |
| DEP-012 | Demo environment is reproducible | Documentation + scripts | Final demo checklist |

---

# 9. Observability Traceability

| OBS ID | Requirement | Implementation | Validation |
|---|---|---|---|
| OBS-REQ-001 | Structured kernel log levels | Logger | TEST-003 |
| OBS-REQ-002 | Kernel errors produce diagnostics | Diagnostics | TEST-008/012 |
| OBS-REQ-003 | Panic path attempts diagnostic output | Panic handler | TEST-008 |
| OBS-REQ-004 | Communication events are traceable | Protocol logs | Protocol tests |
| OBS-REQ-005 | AI requests have identifiers | Request ID | AI observability test |
| OBS-REQ-006 | AI latency is measurable | Telemetry | AI timing test |
| OBS-REQ-007 | AI errors differ from kernel errors | Error taxonomy | Failure tests |
| OBS-REQ-008 | Credentials are not logged | Logging policy | Security scan |
| OBS-REQ-009 | Diagnostics include severity/subsystem | Diagnostic record | Diagnostic tests |
| OBS-REQ-010 | Core observability works without AI | Kernel logger | AI outage test |
| OBS-REQ-011 | Provider/model can be recorded | AI telemetry | AI evaluation |
| OBS-REQ-012 | End-to-end diagnostic path is traceable | IDs/logs | Integration test |

---

# 10. Test Traceability

Tests should map back to requirements rather than existing only as isolated checks.

| Test ID | Scenario | Requirement(s) Covered |
|---|---|---|
| TEST-001 | Kernel boot | BR-001, TR-001, TR-004, DEP-003 |
| TEST-002 | Serial initialization/output | TR-005, DEP-004, OBS-REQ-001 |
| TEST-003 | Logger output | BR-003, TR-006, OBS-REQ-001 |
| TEST-004 | Unknown command | BR-002, TR-013, SEC-001 |
| TEST-005 | Valid command | BR-002, TR-013, TR-014 |
| TEST-006 | Keyboard input | TR-009 |
| TEST-007 | Invalid arguments | TR-013, TR-014, SEC-001 |
| TEST-008 | Controlled panic | BR-004, TR-011, TR-012, OBS-REQ-003 |
| TEST-009 | CPU exception | TR-007, TR-012 |
| TEST-010 | Hardware interrupt | TR-008 |
| TEST-011 | Valid allocation | TR-010 |
| TEST-012 | Allocation failure | BR-004, TR-010, TR-012 |
| TEST-013 | Valid diagnostic message | BR-005, BR-006, TR-017, TR-018, TR-026 |
| TEST-014 | Malformed protocol message | TR-018, TR-025, API-008, SEC-003 |
| TEST-015 | AI request generation | BR-006, BR-007, TR-019, API-001 |
| TEST-016 | Valid AI response | BR-008, TR-021, API-004 |
| TEST-017 | Malformed AI response | TR-021, API-004, SEC-005, SEC-014 |
| TEST-018 | AI timeout | BR-011, TR-024, API-005, DEP-008 |
| TEST-019 | Safe AI command | BR-009, TR-022, API-006 |
| TEST-020 | Privileged/destructive suggestion | BR-010, TR-023, API-006, SEC-006, SEC-007 |
| TEST-021 | Prompt injection | Security Design, SEC-008 |
| TEST-022 | Missing AI credentials | API-003, SEC-009 |
| TEST-023 | AI Bridge unavailable | BR-011, TR-024, API-010, SEC-013 |
| TEST-024 | Voice fallback | BR-013, TR-030 |
| TEST-025 | Repeated AI requests | API-011, API-013, SEC-012 |

---

# 11. Documentation Traceability

The following documents describe different parts of the same system:

| Document | Primary Purpose | Relationship |
|---|---|---|
| BRD | Business/system requirements | Defines why and what |
| PRD | Product requirements | Defines product behavior and scope |
| UX | User interaction requirements | Defines shell/user experience |
| TRD | Technical requirements | Defines technical constraints and choices |
| HLD | High-level architecture | Defines major components and relationships |
| Data Design | Data structures and lifecycle | Defines runtime/protocol data |
| API Specification | External/bridge interface | Defines communication contracts |
| LLD | Low-level implementation design | Defines modules and implementation details |
| GenAI Architecture | AI workflow | Defines AI-specific design |
| Security Design | Security controls | Defines trust and safety boundaries |
| Testing Strategy | Verification strategy | Defines how requirements are tested |
| CI/CD | Automated validation/build | Defines delivery process |
| Observability | System visibility | Defines logs/metrics/tracing |
| Deployment | Runtime/build environment | Defines execution and release |
| Cost Analysis | Cost boundaries | Defines resource/cost assumptions |
| Roadmap | Delivery schedule | Defines implementation order |
| Team Responsibilities | Ownership | Defines who implements/reviews what |
| ADRs | Architecture decisions | Records why major decisions were made |

---

# 12. Requirement Status Model

Use the following status values:

```text
PLANNED
IN PROGRESS
IMPLEMENTED
TESTED
VERIFIED
DEFERRED
REJECTED
SUPERSEDED
```

Recommended progression:

```text
PLANNED
   ↓
IN PROGRESS
   ↓
IMPLEMENTED
   ↓
TESTED
   ↓
VERIFIED
```

A requirement should not be marked `VERIFIED` solely because the code compiles.

---

# 13. Evidence Types

Acceptable requirement evidence may include:

- Successful build output.
- QEMU serial logs.
- Automated test results.
- Kernel screenshots.
- QEMU screenshots.
- AI Bridge test results.
- Protocol fixtures.
- Security scan output.
- CI workflow results.
- Performance measurements.
- Final demonstration recording.
- Git commit/tag references.

Evidence should identify the corresponding build or commit where practical.

---

# 14. Traceability During Development

When implementing a feature:

```text
Requirement
    ↓
Find Design
    ↓
Identify Module
    ↓
Implement
    ↓
Create/Update Test
    ↓
Capture Evidence
    ↓
Update RTM
```

Example:

```text
TR-012
Structured diagnostics
        ↓
LLD Diagnostic Module
        ↓
kernel/src/diagnostics/
        ↓
Diagnostic Record
        ↓
TEST-008 / TEST-012
        ↓
QEMU Diagnostic Output
```

---

# 15. Requirement Change Management

When a requirement changes:

```text
Requirement Change
       ↓
Impact Analysis
       ↓
Check HLD
       ↓
Check LLD
       ↓
Check API / Security / Testing
       ↓
Update Implementation
       ↓
Update Tests
       ↓
Update RTM
```

The team should not change a requirement in one document while leaving dependent documentation unchanged.

---

# 16. Orphan Requirement Detection

A requirement is considered an orphan when it has no implementation or validation path.

Example:

```text
Requirement
     ↓
No Design
     ↓
No Code
     ↓
No Test
```

Such requirements must be:

- Implemented.
- Explicitly deferred.
- Removed/rejected.
- Clarified.

---

# 17. Orphan Test Detection

A test that does not trace back to a requirement should be reviewed.

```text
Test
 ↓
No Requirement
```

This does not automatically mean the test is useless. It may represent:

- Regression testing.
- Internal safety testing.
- Implementation-specific validation.

However, its purpose should be documented.

---

# 18. Architecture Traceability

Major architectural decisions should map to ADRs.

Examples:

```text
Rust kernel
   ↓
ADR-001

no_std
   ↓
ADR-002

x86_64
   ↓
ADR-003

QEMU
   ↓
ADR-004

Host-side AI Bridge
   ↓
ADR-006

Serial communication
   ↓
ADR-007

AI untrusted
   ↓
ADR-008
```

This creates a clear relationship between:

```text
Requirement
 ↓
Architecture
 ↓
Decision
```

---

# 19. MVP Traceability

The most important MVP path is:

```text
Boot
 ↓
Kernel
 ↓
Shell
 ↓
Diagnostics
 ↓
Communication
 ↓
AI Bridge
 ↓
External AI
 ↓
Validated Response
 ↓
Shell
```

Each stage must have:

```text
Requirement
+
Design
+
Implementation
+
Test
+
Evidence
```

---

# 20. Final Traceability Matrix

| Area | Requirements | Design | Implementation | Test | Evidence |
|---|---:|---:|---:|---:|---|
| Boot | ✓ | ✓ | ✓ | ✓ | QEMU log |
| Kernel Init | ✓ | ✓ | ✓ | ✓ | Serial output |
| Serial | ✓ | ✓ | ✓ | ✓ | Host terminal |
| Memory | ✓ | ✓ | ✓ | ✓ | Memory tests |
| Interrupts | ✓ | ✓ | ✓ | ✓ | Exception/interrupt tests |
| Keyboard | ✓ | ✓ | ✓ | ✓ | Shell interaction |
| Shell | ✓ | ✓ | ✓ | ✓ | QEMU screenshot/log |
| Diagnostics | ✓ | ✓ | ✓ | ✓ | Diagnostic record |
| Communication | ✓ | ✓ | ✓ | ✓ | Protocol test |
| AI Bridge | ✓ | ✓ | ✓ | ✓ | Bridge test |
| External AI | ✓ | ✓ | ✓ | ✓ | Integration result |
| AI Validation | ✓ | ✓ | ✓ | ✓ | Validation test |
| Security | ✓ | ✓ | ✓ | ✓ | Security tests |
| CI/CD | ✓ | ✓ | ✓ | ✓ | CI result |
| Observability | ✓ | ✓ | ✓ | ✓ | Logs/metrics |
| Deployment | ✓ | ✓ | ✓ | ✓ | Build/demo |
| Voice | Optional | ✓ | Optional | Optional | Optional |

---

# 21. Final Verification Checklist

Before final project submission:

- [ ] Every mandatory BRD requirement maps to at least one technical/design element.
- [ ] Every mandatory technical requirement maps to implementation.
- [ ] Every critical security requirement maps to a test.
- [ ] Every major component has relevant tests.
- [ ] Core MVP requirements have recorded evidence.
- [ ] HLD and LLD match the implemented architecture.
- [ ] API and protocol documentation match implementation.
- [ ] Security documentation matches actual controls.
- [ ] Testing documentation reflects actual test cases.
- [ ] Roadmap status reflects actual completion.
- [ ] Deferred features are explicitly marked.
- [ ] No mandatory requirement is silently omitted.
- [ ] No major implemented feature exists without documentation or validation.

---

# 22. Definition of Traceability Complete

Psydian traceability is complete when the team can answer:

```text
Why are we building this?
        ↓
BRD / PRD

What should it do?
        ↓
Requirements

How is it designed?
        ↓
HLD / LLD

Where is it implemented?
        ↓
Source Modules

How do we know it works?
        ↓
Tests

How do we prove it?
        ↓
Evidence
```

The RTM provides the connection between these layers and acts as the final verification map for the Psydian MVP.
