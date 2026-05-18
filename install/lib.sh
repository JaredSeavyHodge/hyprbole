#!/bin/bash

set -euo pipefail

log_step() {
  printf '\n==> %s\n' "$1"
}

log_info() {
  printf '  -> %s\n' "$1"
}

die() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

cmd_present() {
  command -v "$1" >/dev/null 2>&1
}

require_command() {
  cmd_present "$1" || die "required command missing: $1"
}

require_sudo() {
  if ! sudo -v; then
    die "sudo access is required. If your correct password is rejected, wait for faillock to expire or reset it from a root shell."
  fi
}

require_interactive_terminal() {
  [[ -t 0 && -t 1 ]] || die "run this installer from an interactive terminal, not through a pipe or non-interactive shell"
}

warn_faillock() {
  cmd_present faillock || return 0

  local failures
  failures=$(faillock --user "$USER" 2>/dev/null || true)

  if printf '%s\n' "$failures" | grep -Eq '[[:space:]]V$'; then
    printf 'warning: valid authentication failures are recorded for %s\n' "$USER" >&2
    printf '%s\n' "$failures" >&2
    printf 'warning: Arch defaults lock PAM auth after 3 failures for 10 minutes. This affects sudo, SDDM, and hyprlock.\n' >&2
  fi
}

start_sudo_keepalive() {
  while true; do
    sudo -n -v >/dev/null 2>&1 || exit 0
    sleep 60
  done &

  HYPRBOLE_SUDO_KEEPALIVE_PID=$!
}

stop_sudo_keepalive() {
  if [[ -n ${HYPRBOLE_SUDO_KEEPALIVE_PID:-} ]]; then
    kill "$HYPRBOLE_SUDO_KEEPALIVE_PID" >/dev/null 2>&1 || true
  fi
}

prompt_value() {
  local prompt="$1"
  local default_value="${2:-}"
  local value=""

  if [[ -n $default_value ]]; then
    read -r -p "$prompt [$default_value]: " value
    if [[ -z $value ]]; then
      value="$default_value"
    fi
  else
    while [[ -z $value ]]; do
      read -r -p "$prompt: " value
    done
  fi

  printf '%s' "$value"
}

copy_if_missing() {
  local source_path="$1"
  local destination_path="$2"

  mkdir -p "$(dirname "$destination_path")"

  if [[ ! -e $destination_path ]]; then
    cp "$source_path" "$destination_path"
    return 0
  fi

  return 1
}
