hyprbole_installed_packages() {
  local package

  for package in "$@"; do
    if pacman -Q "$package" >/dev/null 2>&1; then
      printf '%s\n' "$package"
    fi
  done
}

remove_conflicting_packages() {
  local installed_packages=()
  local package

  while IFS= read -r package; do
    [[ -n $package ]] || continue
    installed_packages+=("$package")
  done < <(hyprbole_installed_packages "${HYPRBOLE_CONFLICTING_PACKAGES[@]}")

  if ((${#installed_packages[@]} == 0)); then
    log_info "no conflicting packages found"
    return 0
  fi

  log_info "removing conflicting package(s): ${installed_packages[*]}"
  sudo pacman -Rns --noconfirm "${installed_packages[@]}"
}

warn_profile_leftovers() {
  local installed_packages=()
  local package

  while IFS= read -r package; do
    [[ -n $package ]] || continue
    installed_packages+=("$package")
  done < <(hyprbole_installed_packages "${HYPRBOLE_PROFILE_LEFTOVER_PACKAGES[@]}")

  if ((${#installed_packages[@]} == 0)); then
    return 0
  fi

  log_info "profile leftover package(s) still installed: ${installed_packages[*]}"
  log_info "Hyprbole does not require these; remove them manually if you do not use them"
}

install_official_packages() {
  local pacman_args=(-Syu --needed --noconfirm)

  if [[ -x ${HYPRBOLE_REPO_ROOT:-}/bin/hyprbole-health-check ]]; then
    "$HYPRBOLE_REPO_ROOT/bin/hyprbole-health-check" --disk-space-guard
  fi

  sudo pacman "${pacman_args[@]}" "${HYPRBOLE_OFFICIAL_PACKAGES[@]}"
}
