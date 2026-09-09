# Cost Analysis

## 1. Purpose

The Cost Analysis section defines the expected financial and resource costs of developing and demonstrating Psydian.

Psydian is primarily an academic operating-system prototype. Its kernel, QEMU environment, Rust toolchain, and most development tools can be used without direct infrastructure charges. The main variable cost is expected to come from external AI API usage and, if selected, optional cloud services.

The purpose of this analysis is to:

- Identify possible project costs.
- Separate fixed and variable costs.
- Avoid assuming unnecessary paid infrastructure.
- Estimate AI-related usage costs using configurable parameters.
- Identify opportunities to reduce cost.
- Keep the MVP financially practical within the 8-week timeline.

---

# 2. Cost Categories

The major cost categories are:

```text
Development Tools
      +
Compute
      +
Storage
      +
Networking
      +
AI / LLM Usage
      +
Optional Voice Services
      +
Deployment / Hosting
```

For the MVP, several of these can remain zero-cost by using local development resources.

---

# 3. Development Tooling Cost

The core development stack consists of:

```text
Rust
Cargo
QEMU
Git
GitHub
Linux
VS Code / IDE
```

These tools can generally be used without direct software-license costs for the academic project.

The project should avoid introducing paid software where an adequate free/open-source alternative exists.

---

# 4. Local Compute Cost

Psydian kernel development primarily occurs on the local development machine.

The local machine provides:

- CPU.
- RAM.
- Disk storage.
- QEMU execution.
- Rust compilation.
- AI Bridge execution.

There is no requirement for dedicated cloud compute for the core MVP.

Therefore, the direct incremental cloud-compute cost for kernel development can be:

```text
MVP Cloud Kernel Compute = ₹0
```

This assumes development uses existing team hardware.

---

# 5. QEMU Cost

QEMU is used as the primary virtual-machine environment for the kernel.

The project does not require paid QEMU infrastructure.

Conceptually:

```text
Existing Laptop / Desktop
        ↓
      QEMU
        ↓
Psydian Kernel
```

Therefore:

```text
QEMU Software Cost = ₹0
```

The actual electricity usage of the development machine is not included in the academic project budget.

---

# 6. Storage Cost

Local storage is required for:

- Source code.
- Rust build artifacts.
- Dependencies.
- Bootable images.
- Test logs.
- AI Bridge artifacts.
- Documentation.

For the MVP, local disk storage should be sufficient.

Potential future costs may arise if the team chooses to use:

- Cloud object storage.
- Remote artifact storage.
- Persistent log storage.
- Hosted diagnostic history.

These are not required for the initial MVP.

---

# 7. GitHub Cost

GitHub is used for:

- Source-code version control.
- Pull requests.
- Documentation.
- Issue tracking.
- CI/CD.
- Release artifacts where appropriate.

The project should use the available GitHub plan/resources without introducing unnecessary paid infrastructure.

The direct project cost can therefore remain:

```text
GitHub MVP Cost = ₹0
```

assuming the existing available account/plan is sufficient.

---

# 8. AI / LLM Cost Model

AI usage is the main potentially variable project cost.

The exact cost depends on:

- Number of AI requests.
- Average input tokens.
- Average output tokens.
- Selected model.
- Provider pricing.
- Retry frequency.
- Context size.

The basic formula is:

```text
Monthly AI Cost
=
Requests × Average Input Tokens × Input Price
+
Requests × Average Output Tokens × Output Price
```

Actual provider prices should only be inserted after the AI provider and model are selected.

---

# 9. AI Request Cost Drivers

The following factors can increase AI cost:

```text
More Requests
      ↓
Higher Cost

Larger Diagnostic Context
      ↓
More Input Tokens
      ↓
Higher Cost

Longer Responses
      ↓
More Output Tokens
      ↓
Higher Cost

Repeated Retries
      ↓
Additional Requests
      ↓
Higher Cost
```

Therefore, the AI Bridge should keep requests focused and bounded.

---

# 10. AI Cost Optimization

The project should control AI cost through:

- Compact prompts.
- Relevant diagnostic context only.
- Maximum request size.
- Maximum response size.
- Bounded retries.
- Request timeouts.
- Caching repeated analyses where useful.
- Smaller models when quality is sufficient.
- Avoiding unnecessary AI calls.
- Avoiding duplicate diagnostic submissions.

The kernel should not continuously send logs to the external AI service.

Only relevant diagnostic events or explicit user requests should trigger AI processing.

---

# 11. AI Request Strategy

Instead of:

```text
Every Kernel Log
      ↓
AI Request
```

use:

```text
Kernel Events
      ↓
Filter
      ↓
Relevant Diagnostic
      ↓
AI Request
```

For normal operation:

```text
INFO logs
   ↓
Local logging only

ERROR / PANIC
   ↓
Potential AI analysis
```

This substantially reduces unnecessary AI usage.

---

# 12. Optional Voice Cost

Voice integration may introduce additional costs depending on the selected speech-to-text and text-to-speech services.

Potential cost sources include:

- Speech-to-text API usage.
- Text-to-speech API usage.
- Local model compute.
- Cloud inference.

For the MVP:

```text
Voice Integration = Optional
```

Therefore, voice-related costs should not be required for the core project budget.

If voice is implemented using a local/open-source solution, direct API cost may remain zero while local compute requirements increase.

---

# 13. Hosting Cost

The Psydian kernel itself does not require cloud hosting for the MVP.

The host-side AI Bridge runs locally:

```text
Linux Host
   ↓
AI Bridge
```

A hosted AI Bridge may be considered later, but it is not required for the initial academic demonstration.

Therefore:

```text
MVP Kernel Hosting Cost = ₹0
MVP AI Bridge Hosting Cost = ₹0
```

assuming both run on existing development hardware.

---

# 14. Networking Cost

The kernel does not require direct external network connectivity for the MVP.

External network communication is handled by:

```text
AI Bridge
   ↓
Host Network
   ↓
HTTPS
   ↓
AI Provider
```

The incremental networking cost is therefore primarily the normal internet connection already available to the development environment.

No dedicated networking infrastructure is required.

---

# 15. Database Cost

The Psydian MVP does not require a conventional database.

Therefore:

```text
Database Hosting Cost = ₹0
```

for the core MVP.

A database may become useful in future versions for:

- Persistent diagnostic history.
- User profiles.
- System telemetry.
- Remote monitoring.
- AI interaction history.

Such infrastructure would introduce additional storage and hosting costs.

---

# 16. Cloud Deployment Cost

A cloud deployment is optional for the MVP.

Possible future components include:

```text
Cloud VM
   ↓
AI Bridge
   ↓
External AI API

Object Storage
   ↓
Diagnostic History

Database
   ↓
Persistent Metadata
```

These should be introduced only if a concrete project requirement exists.

---

# 17. Cost Scenarios

## Scenario A — Local MVP

```text
Linux Host
+
Rust
+
QEMU
+
GitHub
+
Local AI Bridge
+
External AI API
```

Expected direct infrastructure cost:

```text
Low / usage-dependent
```

The main variable component is AI API usage.

---

## Scenario B — Local Development + Hosted AI

```text
Local Kernel
     ↓
Local AI Bridge
     ↓
External Hosted AI
```

Expected cost sources:

```text
AI API usage
+
Existing internet connection
```

No dedicated server is required.

---

## Scenario C — Fully Hosted Extension

```text
Psydian
   ↓
Network
   ↓
Hosted AI Bridge
   ↓
AI Provider
```

Potential costs:

```text
Cloud compute
+
Storage
+
Monitoring
+
Network traffic
+
AI API
```

This is future scope rather than MVP scope.

---

# 18. AI Cost Estimation Template

Once the provider is selected, use the following parameters:

| Parameter | Value |
|---|---:|
| Requests per month | TBD |
| Average input tokens/request | TBD |
| Average output tokens/request | TBD |
| Input token price | TBD |
| Output token price | TBD |
| Estimated monthly AI cost | TBD |

Formula:

```text
Monthly Cost
=
Requests × Input Tokens × Input Price
+
Requests × Output Tokens × Output Price
```

Use the provider's current published pricing when completing the final budget.

---

# 19. Cost per Diagnostic Request

A more useful project metric is cost per AI-assisted diagnostic.

Conceptually:

```text
Cost / Diagnostic
=
Input Token Cost
+
Output Token Cost
+
Retry Cost
```

Example evaluation:

```text
Diagnostic Request
      ↓
Average Input Tokens
      ↓
Average Output Tokens
      ↓
Provider Pricing
      ↓
Cost / Diagnostic
```

This metric can be used during model comparison.

---

# 20. AI Model Selection vs Cost

The cheapest model is not automatically the best choice.

The selection should balance:

```text
Diagnostic Accuracy
+
Groundedness
+
Latency
+
Cost
+
Structured Output Reliability
```

A larger model may provide better reasoning but increase:

- Cost.
- Latency.
- Resource requirements.

A smaller model may reduce cost but produce weaker diagnostic analysis.

The final model selection should therefore be based on measured project results.

---

# 21. Cost Optimization Targets

The project should aim to:

- Keep diagnostic prompts compact.
- Avoid unnecessary context.
- Avoid sending complete log histories.
- Limit AI calls to meaningful requests.
- Limit retry attempts.
- Use the smallest model that meets the required quality.
- Cache repeated non-changing analyses where practical.
- Keep voice optional.
- Avoid unnecessary cloud infrastructure.

---

# 22. Development Cost Assumptions

This project assumes:

- Team members already have access to development computers.
- Linux is available.
- Required open-source development tools can be installed locally.
- QEMU runs locally.
- Git/GitHub access is available.
- External AI API usage, if paid, is controlled and limited.

Hardware purchase cost is therefore excluded from the MVP cost model.

---

# 23. Hidden / Operational Costs

The following costs may exist but are not part of the direct project budget:

- Electricity.
- Existing internet subscription.
- Existing laptop/desktop hardware.
- Developer time.
- Academic supervision.
- Existing software subscriptions.

These should be documented separately if a formal financial analysis is required.

---

# 24. Cost Risks

| Risk | Impact | Mitigation |
|---|---|---|
| High AI API usage | Medium | Request limits, compact prompts, caching |
| Large diagnostic context | Medium | Context filtering and truncation |
| Excessive retries | Medium | Bounded retry policy |
| Expensive AI model | Medium | Compare smaller models |
| Voice API charges | Medium | Keep voice optional |
| Unnecessary cloud hosting | Low | Prefer local development |
| Unexpected provider pricing changes | Medium | Keep pricing configurable |
| Excessive logging/storage | Low | Retention limits and local filtering |

---

# 25. Cost and Privacy Trade-Off

Reducing AI cost should not mean sending insufficient context for useful diagnostics.

The correct optimization is:

```text
Relevant Context
      ↓
Compact Representation
      ↓
Useful AI Analysis
```

not:

```text
Send Everything
      ↓
Large Prompt
      ↓
High Cost
```

and not:

```text
Send Almost Nothing
      ↓
Poor Analysis
```

The goal is to find the smallest context that still supports accurate analysis.

---

# 26. MVP Cost Boundary

The core MVP should require only:

```text
Existing Development Hardware
+
Open-source Development Tools
+
QEMU
+
Git/GitHub
+
Optional / Controlled AI API Usage
```

No dedicated:

- Cloud GPU.
- Database server.
- Object storage.
- Kubernetes cluster.
- Production server.
- Enterprise observability platform.

is required for the MVP.

---

# 27. Future Cost Considerations

If Psydian evolves beyond the academic prototype, possible future infrastructure costs include:

- Cloud AI inference.
- Hosted AI Bridge.
- Database hosting.
- Diagnostic storage.
- Monitoring.
- Remote telemetry.
- Voice services.
- Dedicated compute.
- Physical hardware testing.
- Multi-user infrastructure.

These costs should be evaluated only when those features become actual requirements.

---

# 28. Cost Analysis Requirements

| ID | Requirement | Priority |
|---|---|---|
| COST-001 | AI provider pricing shall remain configurable until a provider is selected. | Must |
| COST-002 | AI request volume shall be bounded. | Must |
| COST-003 | AI requests shall send only relevant diagnostic context. | Must |
| COST-004 | Retry behavior shall be bounded to control cost. | Must |
| COST-005 | Core MVP shall not depend on paid cloud infrastructure. | Should |
| COST-006 | Voice-related costs shall remain optional. | Should |
| COST-007 | Actual provider pricing shall be documented before final budget estimation. | Must |
| COST-008 | AI cost per diagnostic should be measured during evaluation. | Should |
| COST-009 | Unnecessary persistent storage infrastructure shall be avoided. | Should |
| COST-010 | Future infrastructure costs shall be separated from MVP costs. | Must |

---

# 29. Final Cost Model

The Psydian MVP is designed to remain low-cost by keeping the kernel and AI Bridge local.

```text
                 MVP COST

Development Tools       → Minimal / Zero
QEMU                    → Zero
Git/GitHub               → Existing resources
Kernel Hosting           → Zero
AI Bridge Hosting        → Zero
Database                 → Zero
Cloud Compute            → Zero
Voice                    → Optional
External AI API          → Usage-dependent
```

The external AI API is the primary variable cost and should be controlled through request limits, concise prompts, bounded retries, and appropriate model selection.

---

# 30. Final Cost Principle

Psydian should follow:

> **Build locally, use external AI selectively, and avoid paid infrastructure unless it provides a measurable benefit to the MVP.**

The 8-week project should prioritize engineering functionality and demonstrable system behavior over unnecessary infrastructure expenditure.

