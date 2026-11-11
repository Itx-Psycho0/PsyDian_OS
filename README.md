# Psydian

**An AI-assisted operating-system prototype written in Rust for x86_64.**

Psydian is a small bare-metal kernel that boots in QEMU, gives you an interactive shell, and turns kernel failures into structured diagnostic records. A host-side AI Bridge (in progress) will read those records over the serial port and ask an external AI service to explain them. The AI layer never runs inside the kernel and never executes commands on its own.

```text
User -> Psydian shell -> kernel -> diagnostic record -> serial channel -> AI Bridge -> AI service
```

## Current status

| Week | Scope | Status |
|---|---|---|
| 1 | Requirements, documentation, toolchain | Done |
| 2 | Bootable kernel, serial output, logging | Done |
| 3 | Exceptions, interrupts, paging, heap | Done |
| 4 | Keyboard and serial input, shell | Done |
| 5 | Logger levels, diagnostics, panic handling, message protocol | Done |
| 6 | Host-side AI Bridge | Not started |
| 7 | External AI integration and safety validation | Not started |
| 8 | Integration, testing, demo | Not started |

## Prerequisites

- Linux (other hosts may work but are untested)
- [rustup](https://rustup.rs/). The nightly toolchain, `rust-src`, `llvm-tools-preview` and the `x86_64-unknown-none` target are installed automatically from `rust-toolchain.toml`.
- QEMU: `qemu-system-x86_64` must be on your `PATH`.

## Build, run, test

```bash
cargo build
```

Builds the kernel and a bootable BIOS disk image.

```bash
cargo run
```

Boots Psydian in a QEMU window. The serial port is attached to your terminal, so you can type commands either in the QEMU window or directly in the terminal.

```bash
cargo run -- --headless
```

Same, without a window (serial console only). Add `--debug` to log CPU exceptions and resets to `qemu.log`.

```bash
cargo ktest
```

Runs every kernel unit and integration test inside QEMU.

## Shell commands

| Command | What it does |
|---|---|
| `help [command]` | List commands, or show how to use one |
| `clear` | Clear the screen |
| `info` | Kernel and system information |
| `meminfo` | Physical memory, frame and heap usage |
| `uptime` | Time since boot |
| `echo [text...]` | Print text |
| `history` | Previously entered commands |
| `logs [count] [level]` | Recent kernel log records |
| `diag [list \| last \| <n> \| send <n>]` | Show or re-send diagnostic records |
| `ai <question...>` | Send a question to the AI Bridge (no bridge yet, so it reports "unavailable") |
| `panic-test` | Trigger a controlled kernel panic (asks for confirmation) |
| `fault-test <scenario>` | Inject a fault: `breakpoint`, `heap`, `page-fault`, `divide-by-zero`, `invalid-opcode`, `stack-overflow` (asks for confirmation) |
| `shutdown` | Stop the system (exits QEMU) |

Tab completes command names, Up/Down browse the history, Ctrl+C abandons the current line and Ctrl+L clears the screen.

## What a diagnostic looks like

Run `fault-test heap` and confirm with `y`:

```text
[ERROR][MEMORY][MEM-003] Heap allocation failed (diag-001)
@PSY {"type":"diagnostic","version":1,"id":"diag-001","payload":{"severity":"error","subsystem":"memory","code":"MEM-003","uptime_ms":2070,"message":"Heap allocation failed","context":{"size":"67108864","align":"8","heap_size":"1048576","heap_used":"0"}}}
```

The first line is the human-readable log record. The line starting with `@PSY ` is a protocol frame for the AI Bridge and only appears on the serial port, never on the screen. The frame format is specified in [Docs/08-api-specification.md](Docs/08-api-specification.md) (section 8.5) and the error codes in [Docs/14-observability.md](Docs/14-observability.md) (section 11).

## Repository layout

```text
Cargo.toml          workspace + host-side launcher package
build.rs            builds the bootable disk image from the kernel
src/main.rs         launcher: runs QEMU for `cargo run` and for kernel tests
kernel/
  src/
    main.rs         kernel entry point
    lib.rs          boot sequence (kernel::init) and test framework
    serial/         COM1 driver
    framebuffer.rs  pixel text console
    console.rs      print!/println! (serial + screen)
    logger/         levels, subsystems, log history
    gdt.rs          GDT and TSS (double-fault stack)
    interrupts.rs   IDT, exception and IRQ handlers
    time.rs         PIT tick counter
    memory/         paging, frame allocator, heap mapping
    allocator/      kernel heap allocator
    input/          key queue, PS/2 keyboard and serial decoders
    shell/          line editor, parser, commands, history, completion
    diagnostics/    structured diagnostic records
    protocol/       serial frames for the AI Bridge
    panic/          panic and fatal-exception capture
  tests/            integration tests (each boots its own kernel in QEMU)
Docs/               project documentation (requirements, design, roadmap)
```

## Documentation

Start with [Docs/01-project-overview.md](Docs/01-project-overview.md) and [Docs/17-roadmap.md](Docs/17-roadmap.md). The design is described in [Docs/06-hld.md](Docs/06-hld.md) and [Docs/09-lld.md](Docs/09-lld.md).

## Team

- **Anurag Singh** — kernel, shell, core OS implementation
- **Aditya Chauhan** — AI Bridge, AI integration, AI testing
