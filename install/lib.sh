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
  sudo -v || die "sudo access is required"
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
