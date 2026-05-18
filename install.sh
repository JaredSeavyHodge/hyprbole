#!/bin/bash

set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)

export HYPRBOLE_REPO_ROOT="$SCRIPT_DIR"
export HYPRBOLE_PATH="${HYPRBOLE_PATH:-$HOME/.local/share/hyprbole}"
export HYPRBOLE_CONFIG_PATH="${HYPRBOLE_CONFIG_PATH:-$HOME/.config/hyprbole}"
export HYPRBOLE_ASSUME_YES=0

source "$SCRIPT_DIR/install/lib.sh"

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
  [[ $EUID -ne 0 ]] || die "run this as your regular user, not root"
  require_command sudo
  require_command pacman
  require_command git
  require_sudo
}

main() {
  parse_args "$@"
  preflight

  log_step "Collecting user metadata"
  local full_name
  local email

  full_name=$(prompt_value "Full name" "$(git config --global user.name 2>/dev/null || true)")
  email=$(prompt_value "Email" "$(git config --global user.email 2>/dev/null || true)")

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
