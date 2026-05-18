verify_installation() {
  local failures=0
  local binary
  local failed_units
  local unit

  for binary in hyprland uwsm waybar ghostty nautilus swaync swayosd-client yay snapper sddm limine-update; do
    if ! cmd_present "$binary"; then
      printf 'missing command: %s\n' "$binary" >&2
      failures=$((failures + 1))
    fi
  done

  for path in \
    "$HOME/.config/hypr/hyprland.lua" \
    "$HOME/.config/waybar/config.jsonc" \
    "$HYPRBOLE_PATH/.git" \
    "$HYPRBOLE_PATH/default" \
    "$HYPRBOLE_PATH/themes"; do
    if [[ ! -e $path ]]; then
      printf 'missing path: %s\n' "$path" >&2
      failures=$((failures + 1))
    fi
  done

  if [[ -f $HOME/.config/hypr/hyprland.conf ]]; then
    printf 'conflicting Hyprland config remains: %s\n' "$HOME/.config/hypr/hyprland.conf" >&2
    printf 'Hyprbole uses hyprland.lua; move or remove hyprland.conf if it is not intentional.\n' >&2
    failures=$((failures + 1))
  fi

  for path in \
    /etc/brave/policies/managed/color.json; do
    if [[ -e $path && ! -L $path ]]; then
      printf 'browser policy should be a symlink: %s\n' "$path" >&2
      failures=$((failures + 1))
    fi
  done

  for unit in pipewire.service pipewire-pulse.service wireplumber.service swayosd-server.service polkit-gnome-agent.service elephant.service walker.service swaync.service; do
    if ! systemctl --user is-enabled "$unit" >/dev/null 2>&1; then
      printf 'user service is not enabled: %s\n' "$unit" >&2
      failures=$((failures + 1))
    elif ! systemctl --user is-active "$unit" >/dev/null 2>&1; then
      printf 'user service is not active: %s\n' "$unit" >&2
      failures=$((failures + 1))
    fi
  done

  failed_units="$(systemctl --user list-units --state=failed --no-legend --plain 2>/dev/null || true)"
  if [[ -n $failed_units ]]; then
    printf 'failed user services remain:\n' >&2
    printf '%s\n' "$failed_units" >&2
    failures=$((failures + 1))
  fi

  if (( failures > 0 )); then
    die "verification failed with $failures issue(s)"
  fi

  log_info "verification passed"
}
