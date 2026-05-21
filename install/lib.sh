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

diagnostic_section() {
  printf '\n## %s\n' "$1"
}

diagnostic_command() {
  local status

  printf '\n$'
  printf ' %q' "$@"
  printf '\n'

  "$@"
  status=$?
  if (( status != 0 )); then
    printf '[exit %s]\n' "$status"
  fi
}

diagnostic_package_check() {
  local package

  for package in "$@"; do
    if pacman -Q "$package" >/dev/null 2>&1; then
      printf 'ok   %s\n' "$package"
    else
      printf 'miss %s\n' "$package"
    fi
  done
}

diagnostic_path_check() {
  local path

  for path in "$@"; do
    if [[ -e $path ]]; then
      printf 'ok   %s\n' "$path"
    else
      printf 'miss %s\n' "$path"
    fi
  done
}

diagnostic_unit_check() {
  local scope="$1"
  local unit
  shift

  for unit in "$@"; do
    if systemctl $scope is-active "$unit" >/dev/null 2>&1; then
      printf 'ok   active  %s\n' "$unit"
    elif systemctl $scope is-enabled "$unit" >/dev/null 2>&1; then
      printf 'warn enabled inactive %s\n' "$unit"
    else
      printf 'miss disabled/inactive %s\n' "$unit"
    fi
  done
}

write_install_diagnostics() {
  local status="$1"
  local diagnostics_file="${HYPRBOLE_INSTALL_DIAGNOSTICS_FILE:-}"
  local previous_errexit=0
  local user_units=(pipewire.service pipewire-pulse.service wireplumber.service swayosd-server.service polkit-gnome-agent.service gnome-keyring-daemon.socket elephant.service walker.service swaync.service)

  [[ -n $diagnostics_file ]] || return 0

  case $- in
    *e*) previous_errexit=1 ;;
  esac
  set +e

  mkdir -p "$(dirname "$diagnostics_file")"

  {
    printf '# Hyprbole Install Diagnostics\n'
    printf 'status=%s\n' "$status"
    printf 'created_at=%s\n' "$(date --iso-8601=seconds 2>/dev/null || date)"
    printf 'log_file=%s\n' "${HYPRBOLE_INSTALL_LOG_FILE:-}"
    printf 'diagnostics_file=%s\n' "$diagnostics_file"
    printf 'repo_root=%s\n' "${HYPRBOLE_REPO_ROOT:-}"
    printf 'hyprbole_path=%s\n' "${HYPRBOLE_PATH:-}"
    printf 'hyprbole_config_path=%s\n' "${HYPRBOLE_CONFIG_PATH:-}"
    printf 'user=%s\n' "${USER:-}"
    printf 'shell=%s\n' "${SHELL:-}"

    diagnostic_section "System"
    [[ -r /etc/os-release ]] && diagnostic_command sed -n '1,40p' /etc/os-release
    diagnostic_command uname -a
    diagnostic_command timedatectl status
    diagnostic_command locale

    diagnostic_section "Disks And Boot"
    diagnostic_command findmnt /
    diagnostic_command findmnt /boot
    diagnostic_command lsblk -f
    diagnostic_path_check /etc/kernel/cmdline /etc/default/limine /boot/limine.conf

    diagnostic_section "Source Checkout"
    diagnostic_path_check "${HYPRBOLE_REPO_ROOT:-}/.git" "${HYPRBOLE_PATH:-}/.git" "${HYPRBOLE_PATH:-}/bin/hyprbole"
    if git_worktree "${HYPRBOLE_REPO_ROOT:-}"; then
      diagnostic_command git -C "$HYPRBOLE_REPO_ROOT" rev-parse --short HEAD
      diagnostic_command git -C "$HYPRBOLE_REPO_ROOT" status --short --branch
    fi
    if git_worktree "${HYPRBOLE_PATH:-}"; then
      diagnostic_command git -C "$HYPRBOLE_PATH" rev-parse --short HEAD
      diagnostic_command git -C "$HYPRBOLE_PATH" status --short --branch
    fi

    diagnostic_section "Package Checks"
    if declare -p HYPRBOLE_OFFICIAL_PACKAGES >/dev/null 2>&1; then
      printf '\nOfficial packages:\n'
      diagnostic_package_check "${HYPRBOLE_OFFICIAL_PACKAGES[@]}"
    fi
    if declare -p HYPRBOLE_AUR_PACKAGES >/dev/null 2>&1; then
      printf '\nAUR packages:\n'
      diagnostic_package_check yay-bin "${HYPRBOLE_AUR_PACKAGES[@]}"
    fi
    if declare -p HYPRBOLE_CONFLICTING_PACKAGES >/dev/null 2>&1; then
      printf '\nConflicting packages that should be absent:\n'
      for package in "${HYPRBOLE_CONFLICTING_PACKAGES[@]}"; do
        if pacman -Q "$package" >/dev/null 2>&1; then
          printf 'warn installed %s\n' "$package"
        else
          printf 'ok   absent %s\n' "$package"
        fi
      done
    fi

    diagnostic_section "Config Paths"
    diagnostic_path_check \
      "$HOME/.config/hypr/hyprland.lua" \
      "$HOME/.config/hypr/hyprland.conf" \
      "$HOME/.config/hyprbole/theme-sources.conf" \
      "$HOME/.config/waybar/config.jsonc" \
      "$HYPRBOLE_PATH/default/systemd/system/hyprbole-health-check.service" \
      "$HYPRBOLE_PATH/default/systemd/system/hyprbole-health-check.timer" \
      "$HOME/.config/swaync/config.json" \
      "$HOME/.config/elephant/menus/hyprbole-fonts.lua" \
      "$HOME/.config/elephant/menus/hyprbole-power-profiles.toml" \
      "$HOME/.config/elephant/menus/hyprbole-remove.toml" \
      "$HOME/.config/xdg-desktop-portal/hyprland-portals.conf" \
      "$HOME/.config/gtk-3.0/settings.ini" \
      "$HOME/.config/gtk-4.0/settings.ini" \
      "$HOME/.config/code-flags.conf" \
      "$HOME/.config/brave-origin-nightly-flags.conf" \
      "$HOME/.config/elephant/menus/hyprbole-tools.toml" \
      "$HOME/.local/share/applications/hyprbole-brave-origin-nightly.desktop" \
      "$HOME/.local/share/applications/hyprbole-disk-usage.desktop" \
      "$HOME/.config/nvim/init.lua" \
      "$HOME/.config/nvim/lua/config/lazy.lua" \
      "$HOME/.config/nvim/lua/plugins/hyprbole-theme.lua" \
      "$HOME/.config/hyprbole/current/theme-name" \
      "$HOME/.config/hyprbole/current/theme/neovim.lua" \
      "$HOME/.config/hyprbole/current/theme/vscode.json" \
      "$HOME/.config/hyprbole/current/browser-policy.json" \
      /etc/brave/policies/managed/color.json

    diagnostic_section "User Services"
    diagnostic_unit_check --user "${user_units[@]}"
    diagnostic_command systemctl --user list-units --state=failed --no-pager

    diagnostic_section "System Services"
    diagnostic_unit_check "" sddm.service polkit.service hyprbole-health-check.timer limine-snapper-sync.service snapper-cleanup.timer snapper-timeline.timer
    diagnostic_command systemctl list-units --state=failed --no-pager

    diagnostic_section "Authentication"
    if cmd_present faillock; then
      diagnostic_command faillock --user "${USER:-}"
    fi

    diagnostic_section "Hyprbole Doctor"
    if [[ -x ${HYPRBOLE_PATH:-}/bin/hyprbole ]]; then
      HYPRBOLE_PATH="$HYPRBOLE_PATH" diagnostic_command "$HYPRBOLE_PATH/bin/hyprbole" doctor --verbose
    else
      printf 'hyprbole command not available\n'
    fi

    diagnostic_section "Installer Verification"
    if declare -F verify_installation >/dev/null 2>&1; then
      ( verify_installation )
      printf '[exit %s]\n' "$?"
    else
      printf 'verify_installation not loaded\n'
    fi

    diagnostic_section "Recent Journals"
    diagnostic_command journalctl -b -p warning --no-pager -n 120
    diagnostic_command journalctl --user -b -p warning --no-pager -n 120
  } >"$diagnostics_file" 2>&1

  ln -sfn "$diagnostics_file" "$(dirname "$diagnostics_file")/latest.diagnostics.txt"

  if (( previous_errexit == 1 )); then
    set -e
  fi
}

cmd_present() {
  command -v "$1" >/dev/null 2>&1
}

git_worktree_root() {
  [[ -n ${1:-} ]] && git -C "$1" rev-parse --show-toplevel 2>/dev/null
}

git_worktree() {
  git_worktree_root "${1:-}" >/dev/null
}

git_checkout() {
  local path="${1:-}"
  local path_root
  local checkout_root

  [[ -n $path && -d $path ]] || return 1
  path_root=$(cd "$path" && pwd -P) || return 1
  checkout_root=$(git_worktree_root "$path_root") || return 1
  checkout_root=$(cd "$checkout_root" && pwd -P) || return 1

  [[ $path_root == "$checkout_root" ]]
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

  if [[ ${HYPRBOLE_ASSUME_YES:-0} == 1 ]]; then
    [[ -n $default_value ]] || die "$prompt is required; configure it before running install with --yes"
    printf '%s' "$default_value"
    return 0
  fi

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
