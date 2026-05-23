#!/bin/bash

set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)

export HYPRBOLE_INSTALL_LOG_DIR="${HYPRBOLE_INSTALL_LOG_DIR:-$HOME/.local/state/hyprbole/install-logs}"
export HYPRBOLE_INSTALL_STARTED_AT="${HYPRBOLE_INSTALL_STARTED_AT:-$(date +%Y%m%d-%H%M%S)}"
export HYPRBOLE_INSTALL_LOG_FILE="${HYPRBOLE_INSTALL_LOG_FILE:-$HYPRBOLE_INSTALL_LOG_DIR/install-$HYPRBOLE_INSTALL_STARTED_AT.log}"
export HYPRBOLE_INSTALL_DIAGNOSTICS_FILE="${HYPRBOLE_INSTALL_DIAGNOSTICS_FILE:-$HYPRBOLE_INSTALL_LOG_DIR/install-$HYPRBOLE_INSTALL_STARTED_AT.diagnostics.txt}"

mkdir -p "$HYPRBOLE_INSTALL_LOG_DIR"
ln -sfn "$HYPRBOLE_INSTALL_LOG_FILE" "$HYPRBOLE_INSTALL_LOG_DIR/latest.log"
ln -sfn "$HYPRBOLE_INSTALL_DIAGNOSTICS_FILE" "$HYPRBOLE_INSTALL_LOG_DIR/latest.diagnostics.txt"

if [[ -z ${HYPRBOLE_INSTALL_LOGGED:-} ]] && [[ -t 1 ]] && command -v script >/dev/null 2>&1; then
  script_command=$(printf '%q ' "$0" "$@")
  exec env HYPRBOLE_INSTALL_LOGGED=1 script -qefc "$script_command" "$HYPRBOLE_INSTALL_LOG_FILE"
fi

export HYPRBOLE_REPO_ROOT="$SCRIPT_DIR"
export HYPRBOLE_PATH="${HYPRBOLE_PATH:-$HOME/.local/share/hyprbole}"
export HYPRBOLE_CONFIG_PATH="${HYPRBOLE_CONFIG_PATH:-$HOME/.config/hyprbole}"
export HYPRBOLE_ASSUME_YES=0
export HYPRBOLE_SUDO_KEEPALIVE_PID=""

source "$SCRIPT_DIR/install/lib.sh"
source "$SCRIPT_DIR/default/hyprbole/manifest.sh"

source "$SCRIPT_DIR/install/packages.sh"
source "$SCRIPT_DIR/install/yay.sh"
source "$SCRIPT_DIR/install/aur.sh"
source "$SCRIPT_DIR/install/git.sh"
source "$SCRIPT_DIR/install/defaults.sh"
source "$SCRIPT_DIR/install/services.sh"
source "$SCRIPT_DIR/install/session.sh"
source "$SCRIPT_DIR/install/verify.sh"

parse_args() {
  while (( $# > 0 )); do
    case "$1" in
      -y|--yes)
        HYPRBOLE_ASSUME_YES=1
        ;;
      *)
        die "unknown argument: $1"
        ;;
    esac
    shift
  done
}

preflight() {
  [[ $EUID -ne 0 ]] || die "run this as your regular user, not with sudo. The installer will ask for sudo when needed."
  require_command sudo
  require_command pacman
  require_command git
  warn_faillock
  require_interactive_terminal
  require_sudo
  start_sudo_keepalive
}

install_exit_trap() {
  local status="$1"

  trap - EXIT
  stop_sudo_keepalive
  write_install_diagnostics "$status"

  if (( status == 0 )); then
    log_info "install log: $HYPRBOLE_INSTALL_LOG_FILE"
    log_info "diagnostics: $HYPRBOLE_INSTALL_DIAGNOSTICS_FILE"
  else
    printf 'Install failed with status %s.\n' "$status" >&2
    printf 'Log: %s\n' "$HYPRBOLE_INSTALL_LOG_FILE" >&2
    printf 'Diagnostics: %s\n' "$HYPRBOLE_INSTALL_DIAGNOSTICS_FILE" >&2
  fi

  exit "$status"
}

main() {
  trap 'install_exit_trap $?' EXIT
  log_info "install log: $HYPRBOLE_INSTALL_LOG_FILE"
  log_info "diagnostics: $HYPRBOLE_INSTALL_DIAGNOSTICS_FILE"
  parse_args "$@"
  preflight

  log_step "Collecting user metadata"
  local full_name
  local email

  full_name=$(prompt_value "Full name" "$(git config --global user.name 2>/dev/null || true)")
  email=$(prompt_value "Email" "$(git config --global user.email 2>/dev/null || true)")

  log_step "Removing conflicting packages"
  remove_conflicting_packages
  warn_profile_leftovers

  log_step "Installing official packages"
  install_official_packages

  log_step "Installing yay"
  install_yay

  log_step "Installing curated AUR packages"
  install_aur_packages

  log_step "Configuring git"
  configure_git_identity "$full_name" "$email"

  log_step "Deploying defaults"
  deploy_defaults

  log_step "Configuring session defaults"
  configure_session

  log_step "Enabling services"
  enable_services

  log_step "Running verification"
  verify_installation

  printf '\nHyprbole bootstrap completed.\n'
}

main "$@"
