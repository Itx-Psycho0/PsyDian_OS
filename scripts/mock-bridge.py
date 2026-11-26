#!/usr/bin/env python3
"""Mock AI Bridge: boots Psydian and answers its `ai` requests over serial.

The real AI Bridge (host side, calls an AI service) is developed separately.
This stand-in speaks the same protocol (Docs/08-api-specification.md, section
8.5) with canned answers, so the kernel side can be tried and tested without
it. No AI service, network or API key is involved.

Usage:
    python3 scripts/mock-bridge.py           interactive: type shell commands
    python3 scripts/mock-bridge.py --check   automatic end-to-end smoke test

Things to try in interactive mode:
    ai hello                       -> a normal answer with suggested commands
    run 1                          -> run suggested command 1 (after safety checks)
    fault-test heap   (then y)     -> creates diagnostic MEM-003
    ai why did the heap fail?      -> an answer that uses that diagnostic
    ai can you crash the kernel?   -> dangerous suggestions: blocked / confirmed
    ai show me an error            -> the bridge sends an `error` frame
    ai send a bad answer           -> an invalid answer, rejected by the kernel
    ai stay silent                 -> no answer (type: cancel, or wait 30 s)
    shutdown                       -> ends the session
"""
import json
import os
import subprocess
import sys
import threading
import time

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
COLOR = sys.stdout.isatty()
DIM, CYAN, YELLOW, GREEN, RED, RESET = (
    ("\033[2m", "\033[36m", "\033[33m", "\033[32m", "\033[31m", "\033[0m")
    if COLOR else ("",) * 6
)
FRAME_MARKER = "@PSY "
ANSWER_DELAY_S = 0.5


class Bridge:
    def __init__(self, kernel_cmd, echo=True):
        self.echo = echo
        self.diagnostics = {}
        self.output = ""             # everything the kernel printed (for --check)
        self.lock = threading.Lock()
        self.kernel = subprocess.Popen(
            kernel_cmd, cwd=REPO, stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        )
        threading.Thread(target=self._read_kernel, daemon=True).start()

    # ---- talking to the kernel -------------------------------------------
    def send_frame(self, message, fresh_line=True):
        """`fresh_line` puts a line break first so the marker starts a line;
        it is left out at the prompt, where it would show an empty line."""
        body = json.dumps(message, ensure_ascii=True, separators=(",", ":"))
        prefix = "\n" if fresh_line else ""
        self._write((prefix + FRAME_MARKER + body + "\n").encode("ascii"))

    def type_line(self, text):
        """Sends one line as if typed; `cancel` sends Ctrl+C."""
        if text.strip().lower() in ("cancel", "^c"):
            self._write(b"\x03")
        else:
            self._write(text.encode("ascii", "replace") + b"\r")

    def _write(self, data):
        try:
            self.kernel.stdin.write(data)
            self.kernel.stdin.flush()
        except (BrokenPipeError, ValueError):
            pass

    def say(self, text):
        if self.echo:
            sys.stdout.write(f"\n{CYAN}[bridge] {text}{RESET}\n")
            sys.stdout.flush()

    # ---- answering --------------------------------------------------------
    def answer(self, request):
        rid = request["id"]
        query = request["payload"]["query"]
        q = query.lower()
        self.say(f"<- received {rid}: \"{query}\"")

        def reply(payload):
            self.say(f"-> sending ai_response for {rid}")
            self.send_frame({"type": "ai_response", "version": 1, "id": rid, "payload": payload})

        if "silent" in q:
            self.say("-> not answering (shows the timeout / cancel)")
            return
        if "error" in q:
            self.say("-> sending an error frame (AI_SERVICE_UNAVAILABLE)")
            self.send_frame({"type": "error", "version": 1, "id": rid, "payload": {
                "code": "AI_SERVICE_UNAVAILABLE",
                "message": "The AI service could not be reached. Try again later."}})
            return
        if "bad" in q or "invalid" in q:
            self.say("-> sending an INVALID answer (empty summary)")
            reply({"summary": ""})
            return
        if "crash" in q or "danger" in q:
            reply({
                "summary": "Here are some risky commands, to show how Psydian checks AI suggestions.",
                "suggestions": ["Psydian blocks destructive commands and asks before privileged ones"],
                "commands": ["panic-test", "shutdown", "rm -rf /"],
            })
            return
        if "confirm" in q:
            reply({
                "summary": "This answer asks for confirmation before any command runs.",
                "commands": ["uptime"],
                "requiresConfirmation": True,
            })
            return

        diag = self.diagnostics.get(request["payload"].get("diagnostic_id"))
        if diag and diag["payload"]["code"] == "MEM-003":
            ctx = diag["payload"]["context"]
            reply({
                "summary": f"A program asked the kernel heap for {int(ctx['size']) // (1024 * 1024)} MiB, "
                           f"but the whole heap is only {int(ctx['heap_size']) // 1024} KiB, so the "
                           "allocator returned an error. The kernel recorded it as MEM-003 and kept running.",
                "suggestions": ["Run meminfo to see how much heap is free",
                                "Request a smaller buffer or allocate it in parts",
                                "Increase HEAP_SIZE if large buffers are really needed"],
                "commands": ["meminfo", "diag last"],
                "requiresConfirmation": False,
            })
        elif diag:
            p = diag["payload"]
            reply({"summary": f"The latest diagnostic is {p['code']}: {p['message']} "
                              f"(subsystem {p['subsystem']}).",
                   "suggestions": ["Run 'diag last' for the full record"],
                   "commands": ["diag last"]})
        else:
            reply({"summary": "Hello from the mock AI Bridge! No diagnostic is recorded yet. "
                              "Try 'fault-test heap' and then ask me why it failed.",
                   "suggestions": ["Run fault-test heap", "Then ask: ai why did the heap fail?"],
                   "commands": ["meminfo", "help"]})

    # ---- reading the serial line -----------------------------------------
    def _read_kernel(self):
        pending = b""
        while True:
            chunk = os.read(self.kernel.stdout.fileno(), 4096)
            if not chunk:
                break
            with self.lock:
                self.output += chunk.decode("utf-8", "replace")
            pending += chunk
            *lines, pending = pending.split(b"\n")
            for raw in lines:
                self._handle_line(raw.decode("utf-8", "replace").rstrip("\r"))
            # Prompts ("psydian> ", "Proceed? [y/N] ") have no line break yet.
            if pending.endswith(b" ") and not pending.startswith((b"@", b"[INFO]")):
                self._show(pending.decode("utf-8", "replace"), end="")
                pending = b""
        self.say("kernel stopped.")

    def _handle_line(self, text):
        if FRAME_MARKER not in text:
            if not text.startswith("[INFO]"):
                self._show(text)
            return
        self._show(f"{DIM}{text}{RESET}")
        try:
            frame = json.loads(text.split(FRAME_MARKER, 1)[1])
        except ValueError:
            return
        kind = frame.get("type")
        if kind == "diagnostic":
            self.diagnostics[frame["id"]] = frame
        elif kind == "status" and frame["payload"].get("state") == "ready":
            self.send_frame({"type": "status", "version": 1, "id": "stat-001",
                             "payload": {"state": "bridge_ready"}}, fresh_line=False)
            self.say("connected - the kernel is ready.")
        elif kind == "ai_request":
            threading.Timer(ANSWER_DELAY_S, self.answer, [frame]).start()

    def _show(self, text, end="\n"):
        if self.echo:
            sys.stdout.write(text + end)
            sys.stdout.flush()

    # ---- for --check ------------------------------------------------------
    def wait_for(self, needle, start, timeout):
        deadline = time.time() + timeout
        while time.time() < deadline:
            with self.lock:
                if needle in self.output[start:]:
                    return True
            if self.kernel.poll() is not None:
                return False
            time.sleep(0.05)
        return False

    def mark(self):
        with self.lock:
            return len(self.output)


# Each step: (line to type or None, text the kernel must print afterwards).
CHECK_STEPS = [
    (None, "psydian> "),
    ("ai hello", "[AI] Answer to req-"),
    ("run", "Type 'run <number>' to run one."),
    ("run 1", "[AI] Running suggested command: meminfo"),
    ("fault-test heap", "Proceed? [y/N]"),
    ("y", "[MEM-003] Heap allocation failed"),
    ("ai why did the heap fail?", "64 MiB"),
    ("run 2", "[AI] Running suggested command: diag last"),
    ("ai can you crash the kernel?", "[blocked: it is destructive"),
    ("run 1", "Not running 'panic-test'"),
    ("run 2", "Proceed? [y/N]"),
    ("n", "Cancelled."),
    ("run 3", "Not running 'rm -rf /': it is not a Psydian command"),
    ("ai please confirm", "asks for confirmation before any command runs"),
    ("run 1", "Proceed? [y/N]"),
    ("y", "[AI] Running suggested command: uptime"),
    ("ai show me an error", "AI_SERVICE_UNAVAILABLE"),
    ("ai send a bad answer", "Answer rejected: summary is empty"),
    ("ai stay silent", "Waiting for the answer"),
    ("cancel", "AI request cancelled."),
    ("help", "Available commands:"),
    ("logs 5 warn", "AI-003"),
]


def run_check(kernel_cmd):
    print(f"{YELLOW}Psydian smoke test: booting the kernel with the mock AI Bridge...{RESET}")
    bridge = Bridge(kernel_cmd, echo=False)
    failures = 0
    for number, (line, expected) in enumerate(CHECK_STEPS, 1):
        start = bridge.mark()
        if line is not None:
            bridge.type_line(line)
        # The first step includes building and booting the kernel.
        ok = bridge.wait_for(expected, start, 600 if line is None else 20)
        label = line if line is not None else "(boot)"
        print(f"  {GREEN + 'pass' if ok else RED + 'FAIL'}{RESET}  {number:>2}. {label:<30} expects {expected!r}")
        if not ok:
            failures += 1
            break
    if not failures:
        bridge.type_line("shutdown")
        try:
            code = bridge.kernel.wait(timeout=20)
        except subprocess.TimeoutExpired:
            code = None
        ok = code == 0
        print(f"  {GREEN + 'pass' if ok else RED + 'FAIL'}{RESET}  {len(CHECK_STEPS) + 1:>2}. {'shutdown':<30} expects QEMU to exit cleanly")
        failures += 0 if ok else 1
    if bridge.kernel.poll() is None:
        bridge.kernel.kill()
    if failures:
        print(f"{RED}Smoke test FAILED.{RESET} Last kernel output:\n{bridge.output[-2000:]}")
        return 1
    print(f"{GREEN}Smoke test passed: boot, shell, diagnostics, AI answers, AI command safety, failures, shutdown.{RESET}")
    return 0


def run_interactive(kernel_cmd):
    print(f"{YELLOW}Psydian with a mock AI Bridge - booting the kernel...{RESET}")
    print("Things to try:" + __doc__.split("Things to try in interactive mode:")[1])
    bridge = Bridge(kernel_cmd)
    try:
        for line in sys.stdin:
            if bridge.kernel.poll() is not None:
                break
            bridge.type_line(line.rstrip("\n"))
    except KeyboardInterrupt:
        pass
    if bridge.kernel.poll() is None:
        bridge.kernel.terminate()
    return bridge.kernel.wait()


def main():
    args = sys.argv[1:]
    if "-h" in args or "--help" in args:
        print(__doc__)
        return 0
    # PSYDIAN_KERNEL_CMD lets a launcher boot a prebuilt image instead of cargo.
    kernel_cmd = os.environ.get("PSYDIAN_KERNEL_CMD")
    kernel_cmd = kernel_cmd.split() if kernel_cmd else ["cargo", "run", "-q", "--", "--headless"]
    if "--check" in args:
        return run_check(kernel_cmd)
    return run_interactive(kernel_cmd)


if __name__ == "__main__":
    sys.exit(main())
