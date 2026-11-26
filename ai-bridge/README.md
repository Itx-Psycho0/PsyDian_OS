# Psydian AI Bridge

The host-side program between the Psydian kernel and an AI service. The kernel never talks to the network and never sees an API key. The bridge reads the kernel's serial frames, asks the AI and sends back one checked answer per question.

```text
Psydian kernel ──serial (@PSY frames)──► AI Bridge ──HTTPS──► AI service (Claude by default)
      ▲                                      │
      └──── ai_response / error frame ◄──────┘
```

## Quick start

From the repository root, with the `psydian` command installed:

```bash
export ANTHROPIC_API_KEY="your-api-key"
```

```bash
psydian ai
```

Then, in the Psydian shell:

```text
psydian> fault-test heap          (confirm with y)
psydian> ai why did the heap fail?
psydian> run 1
```

`psydian ai --window` does the same with a QEMU window. Type in the window or in the terminal.

**No API key?** Two options:

- `AI_PROVIDER=mock psydian ai` uses canned answers and needs no network.
- `AI_PROVIDER=ollama psydian ai` uses a free local model through [Ollama](https://ollama.com) (`ollama pull llama3.2` first).

**Self-test** (real kernel and bridge, mock provider, no key needed):

```bash
psydian ai-check
```

Without the `psydian` command: `cargo run --manifest-path ai-bridge/Cargo.toml -- --help`.

## Configuration

Everything comes from environment variables. Nothing is stored in the repository.

| Variable | Default | Meaning |
|---|---|---|
| `AI_PROVIDER` | `anthropic` | `anthropic` (Claude), `openai` / `ollama` (OpenAI-style chat API), `mock` |
| `ANTHROPIC_API_KEY` or `AI_API_KEY` | - | API key. Never logged, never sent to the kernel |
| `AI_MODEL` | `claude-opus-5-5` | Model name (`llama3.2` for Ollama) |
| `AI_ENDPOINT` | Anthropic Messages API | Must be `https://`; plain `http://` only for localhost |
| `AI_EFFORT` | `low` | Claude effort: `low`, `medium`, `high`, `xhigh`, `max` |
| `AI_TIMEOUT` | `25` | Seconds per AI call (1-28; the kernel waits 30) |
| `AI_MAX_REQUESTS_PER_MINUTE` | `10` | More requests get `AI_RATE_LIMITED` |
| `PSYDIAN_KERNEL_CMD` | `cargo run -q -- --headless` | How the bridge boots the kernel |

Without an API key the bridge still starts. Every question then gets an `AI_NOT_CONFIGURED` error and the kernel keeps working.

## How a question is answered

1. **protocol**: the kernel's `ai_request` frame is parsed and validated (version 1, `req-NNN` id, query of 1-512 bytes).
2. **diagnostics / context**: the diagnostic the question refers to, plus up to three other recent ones, are taken from the records the kernel reported. Control characters are removed and values are capped.
3. **prompt**: system instructions (versioned `psydian-diag-v1`) and the question. Diagnostic data and the question are wrapped in `<diagnostic>` / `<question>` tags, and the instructions say to ignore any instructions inside them (prompt-injection defence).
4. **provider**: Claude via `POST /v1/messages`:
   - adaptive thinking at the configured effort
   - structured outputs (`output_config.format`, JSON schema)
   - server-side refusal fallback (`fallbacks: "default"`)
   - one retry when the service is overloaded

   A `refusal` stop reason becomes `AI_REFUSED`.
5. **validation**: the answer must be a JSON object with a non-empty `summary`. Text is converted to plain printable ASCII and cut to the kernel's limits (summary 480, at most 4 suggestions of 160, 3 commands of 80). The frame must fit the kernel's 2048-byte buffer.
6. **safety**: suggested commands must be one Psydian command line. Unknown commands (`rm -rf /`), shell syntax (`;`, `|`, `$`…) and destructive commands (`panic-test`, `fault-test`) are dropped. Sensitive or privileged ones (`ai`, `shutdown`) set `requiresConfirmation`. The kernel checks every suggestion again before `run <n>`, so the AI never gets execution authority.
7. The `ai_response` frame goes back to the kernel. Any failure becomes an `error` frame instead.

| Error code | When |
|---|---|
| `AI_NOT_CONFIGURED` | No API key / bad endpoint |
| `AI_AUTH_FAILED` | The service rejected the key (401/403) |
| `AI_RATE_LIMITED` | Service 429, or more than `AI_MAX_REQUESTS_PER_MINUTE` |
| `AI_TIMEOUT` | No answer within `AI_TIMEOUT` |
| `AI_SERVICE_UNAVAILABLE` | Network error or 5xx |
| `AI_BAD_REQUEST` | The service rejected the request (e.g. wrong `AI_MODEL`) |
| `AI_REFUSED` | The model declined to answer |
| `AI_INVALID_RESPONSE` | The answer failed validation or was cut off |

## Request log

`ai-bridge/ai-bridge.log` gets one line per request: request id, provider, prompt version, latency and outcome. API keys, prompts and answers are never written. The file is git-ignored.

## Source layout

```text
src/
  main.rs         CLI, session threads (kernel reader, AI worker, typed input), --check
  kernel.rs       serial link: QEMU on stdio, or QEMU serial over TCP (--window / --connect)
  protocol.rs     @PSY frames: parse kernel frames, build ASCII-only reply frames
  diagnostics.rs  the last 32 diagnostic records
  context.rs      what is sent to the AI for one question
  prompt.rs       system prompt, user message, answer JSON schema
  provider/       Provider trait; anthropic.rs (Claude), openai.rs (OpenAI-style / Ollama), mock.rs
  validation.rs   AI text -> checked Answer within the kernel's limits
  safety.rs       command allowlist and risk levels (mirrors the kernel registry)
  bridge.rs       one request end to end, rate limiting
  telemetry.rs    request log
```

## Testing

```bash
cargo test --manifest-path ai-bridge/Cargo.toml
```
Runs the unit tests: protocol, config, context, prompt, validation, safety, providers, rate limiting.

```bash
psydian ai-check
```
End to end: boots the real kernel and drives answers, `run`, filtered dangerous suggestions, timeout / outage / garbage answers, and shutdown.

CI runs both on every pull request.
