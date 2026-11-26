#!/usr/bin/env bash
# Psydian one-command installer.
#
#   curl -fsSL https://raw.githubusercontent.com/Itx-Psycho0/PsyDian_OS/main/scripts/install.sh | bash
#
# What it does, skipping anything that is already there:
#   1. installs QEMU, git, Python 3, curl and a C linker with the system
#      package manager (apt, dnf, pacman, zypper or Homebrew)
#   2. installs Rust with rustup (the exact nightly toolchain comes from
#      rust-toolchain.toml in the repository)
#   3. downloads Psydian (or updates an existing copy)
#   4. builds the kernel and its bootable disk image
#   5. installs the `psydian` command into ~/.local/bin
#   6. runs the smoke test to prove everything works
#
# Settings (environment variables):
#   PSYDIAN_DIR=~/PsyDian_OS   where to put the source code
#   PSYDIAN_REPO=<git url>     repository to download
#   PSYDIAN_BRANCH=main        branch to download
#   PSYDIAN_SKIP_PACKAGES=1    do not touch system packages
#   PSYDIAN_SKIP_CHECK=1       do not run the smoke test at the end
#
# Nothing outside your home directory is changed except system packages,
# which are installed with sudo after the package manager asks as usual.

set -euo pipefail

# Everything is inside main(), so with `curl ... | bash` the whole script has
# been read before anything runs (a command reading stdin cannot eat it).
main() {

PSYDIAN_DIR="${PSYDIAN_DIR:-$HOME/PsyDian_OS}"
PSYDIAN_REPO="${PSYDIAN_REPO:-https://github.com/Itx-Psycho0/PsyDian_OS.git}"
PSYDIAN_BRANCH="${PSYDIAN_BRANCH:-main}"
BIN_DIR="$HOME/.local/bin"

if [ -t 1 ]; then
  BOLD=$'\033[1m' GREEN=$'\033[32m' YELLOW=$'\033[33m' RED=$'\033[31m' RESET=$'\033[0m'
else
  BOLD="" GREEN="" YELLOW="" RED="" RESET=""
fi
step() { printf '\n%s==> %s%s\n' "$BOLD" "$1" "$RESET"; }
ok()   { printf '    %sok%s  %s\n' "$GREEN" "$RESET" "$1"; }
warn() { printf '    %s!%s   %s\n' "$YELLOW" "$RESET" "$1"; }
die()  { printf '\n%serror:%s %s\n' "$RED" "$RESET" "$1" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

printf '%sPsydian installer%s - AI-assisted operating system prototype (runs in QEMU)\n' "$BOLD" "$RESET"

# --- 1. system packages ------------------------------------------------------
step "Checking system packages"

OS="$(uname -s)"
case "$OS" in
  Linux)  ;;
  Darwin) ;;
  *) die "unsupported system '$OS'. On Windows, install WSL (wsl --install), open Ubuntu and run this command there." ;;
esac

missing=()
have git     || missing+=(git)
have curl    || missing+=(curl)
have python3 || missing+=(python3)
have qemu-system-x86_64 || missing+=(qemu)
have cc      || missing+=(cc)

SUDO=""
if [ "$(id -u)" -ne 0 ]; then
  if have sudo; then SUDO="sudo"; fi
fi

install_packages() {
  if [ "$OS" = Darwin ]; then
    have brew || die "Homebrew is needed on macOS: see https://brew.sh, then run this installer again."
    local pkgs=()
    for item in "$@"; do
      case "$item" in
        qemu) pkgs+=(qemu) ;; git) pkgs+=(git) ;; python3) pkgs+=(python) ;; curl) pkgs+=(curl) ;;
        cc) xcode-select --install 2>/dev/null || true
            warn "install the Xcode command line tools in the window that opened, then run this installer again"
            exit 1 ;;
      esac
    done
    brew install "${pkgs[@]}"
    return
  fi

  if [ "$(id -u)" -ne 0 ] && [ -z "$SUDO" ]; then
    die "please install these packages (no sudo available): $*"
  fi

  local manager pkgs=()
  if have apt-get; then manager=apt
  elif have dnf; then manager=dnf
  elif have pacman; then manager=pacman
  elif have zypper; then manager=zypper
  else die "no supported package manager found. Please install: git curl python3 qemu-system-x86_64 and a C compiler (gcc)."
  fi

  for item in "$@"; do
    case "$manager:$item" in
      apt:qemu)    pkgs+=(qemu-system-x86) ;;
      apt:cc)      pkgs+=(build-essential) ;;
      dnf:qemu)    pkgs+=(qemu-system-x86) ;;
      dnf:cc)      pkgs+=(gcc) ;;
      pacman:qemu) pkgs+=(qemu-system-x86) ;;
      pacman:cc)   pkgs+=(gcc) ;;
      pacman:python3) pkgs+=(python) ;;
      zypper:qemu) pkgs+=(qemu-x86) ;;
      zypper:cc)   pkgs+=(gcc) ;;
      *:*)         pkgs+=("$item") ;;
    esac
  done

  echo "    installing: ${pkgs[*]}"
  case "$manager" in
    apt)    $SUDO apt-get update -qq
            $SUDO env DEBIAN_FRONTEND=noninteractive apt-get install -y -qq "${pkgs[@]}" >/dev/null ;;
    dnf)    $SUDO dnf install -y "${pkgs[@]}" ;;
    pacman) $SUDO pacman -S --needed --noconfirm "${pkgs[@]}" ;;
    zypper) $SUDO zypper --non-interactive install "${pkgs[@]}" ;;
  esac
}

if [ "${#missing[@]}" -eq 0 ]; then
  ok "git, curl, python3, qemu-system-x86_64 and cc are installed"
elif [ "${PSYDIAN_SKIP_PACKAGES:-0}" = 1 ]; then
  die "missing: ${missing[*]} (PSYDIAN_SKIP_PACKAGES=1 is set, so they were not installed)"
else
  install_packages "${missing[@]}"
  for tool in git curl python3 qemu-system-x86_64 cc; do
    have "$tool" || die "$tool is still missing after installing packages"
  done
  ok "system packages installed"
fi

# --- 2. Rust -----------------------------------------------------------------
step "Checking Rust"
if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi
if ! have rustup; then
  echo "    installing rustup (Rust toolchain manager) into ~/.cargo"
  curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain none --no-modify-path
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi
ok "rustup $(rustup --version 2>/dev/null | awk '{print $2}')"

# --- 3. source code ----------------------------------------------------------
step "Getting the Psydian source code"
if [ -d "$PSYDIAN_DIR/.git" ]; then
  git -C "$PSYDIAN_DIR" fetch -q origin "$PSYDIAN_BRANCH"
  if git -C "$PSYDIAN_DIR" merge -q --ff-only FETCH_HEAD 2>/dev/null; then
    ok "updated $PSYDIAN_DIR"
  else
    warn "$PSYDIAN_DIR has local changes; kept them and skipped the update"
  fi
elif [ -e "$PSYDIAN_DIR" ]; then
  die "$PSYDIAN_DIR exists but is not a Psydian checkout. Move it away or set PSYDIAN_DIR."
else
  git clone -q --depth 1 --branch "$PSYDIAN_BRANCH" "$PSYDIAN_REPO" "$PSYDIAN_DIR"
  ok "downloaded to $PSYDIAN_DIR"
fi

# --- 4. build ----------------------------------------------------------------
step "Building the kernel (the first build downloads the Rust nightly toolchain; a few minutes)"
(cd "$PSYDIAN_DIR" && cargo build --quiet)
ok "kernel and bootable disk image built"
if [ -f "$PSYDIAN_DIR/ai-bridge/Cargo.toml" ]; then
  (cd "$PSYDIAN_DIR" && cargo build --quiet --manifest-path ai-bridge/Cargo.toml)
  ok "AI Bridge built"
fi

# --- 5. the psydian command --------------------------------------------------
step "Installing the 'psydian' command"
mkdir -p "$BIN_DIR"
chmod +x "$PSYDIAN_DIR/scripts/psydian" "$PSYDIAN_DIR/scripts/mock-bridge.py"
ln -sf "$PSYDIAN_DIR/scripts/psydian" "$BIN_DIR/psydian"
ok "$BIN_DIR/psydian"

case ":$PATH:" in
  *":$BIN_DIR:"*) on_path=1 ;;
  *) on_path=0 ;;
esac
if [ "$on_path" = 0 ]; then
  # shellcheck disable=SC2016  # $HOME/$PATH must stay literal in the profile
  line='export PATH="$HOME/.local/bin:$PATH"  # added by the Psydian installer'
  added=0
  # .profile for login shells, .bashrc/.zshrc for interactive terminals.
  for rc in "$HOME/.profile" "$HOME/.bashrc" "$HOME/.zshrc"; do
    if [ -f "$rc" ] || [ "$rc" = "$HOME/.profile" ]; then
      if ! grep -qs 'added by the Psydian installer' "$rc"; then
        printf '\n%s\n' "$line" >> "$rc"
        added=1
      fi
    fi
  done
  if [ "$added" = 1 ]; then
    ok "added ~/.local/bin to PATH in your shell profile (open a new terminal to use it)"
  else
    ok "your shell profile already puts ~/.local/bin on PATH (open a new terminal to use it)"
  fi
fi

# --- 6. smoke test -----------------------------------------------------------
if [ "${PSYDIAN_SKIP_CHECK:-0}" != 1 ]; then
  step "Testing the installation"
  "$PSYDIAN_DIR/scripts/psydian" check || die "the smoke test failed; see the output above"
fi

cat <<EOF

${GREEN}${BOLD}Psydian is installed.${RESET}

  psydian            boot Psydian (QEMU window if you have a display, else in this terminal)
  psydian demo       boot with a mock AI Bridge: try 'ai hello', then 'run 1'
  psydian ai         boot with the real AI Bridge (set ANTHROPIC_API_KEY first)
  psydian check      quick end-to-end test (about 30 seconds)
  psydian test       run the full kernel test suite
  psydian update     download the latest version and rebuild
  psydian help       all options

Inside Psydian type 'help' for the shell commands and 'shutdown' to quit.
EOF
if [ "$on_path" = 0 ]; then
  echo "Open a new terminal first, or run it as: $BIN_DIR/psydian"
fi
}

main "$@" < /dev/null
