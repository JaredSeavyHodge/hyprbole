verify_installation() {
  local failures=0
  local binary

  for binary in hyprland uwsm waybar ghostty nautilus swaync swayosd-client yay snapper sddm limine-update; do
    if ! cmd_present "$binary"; then
      printf 'missing command: %s\n' "$binary" >&2
      failures=$((failures + 1))
    fi
  done

  for path in \
    "$HOME/.config/hypr/hyprland.lua" \
    "$HOME/.config/waybar/config.jsonc" \
    "$HYPRBOLE_PATH/default" \
    "$HYPRBOLE_PATH/themes"; do
    if [[ ! -e $path ]]; then
      printf 'missing path: %s\n' "$path" >&2
      failures=$((failures + 1))
    fi
  done

  if (( failures > 0 )); then
    die "verification failed with $failures issue(s)"
  fi

  log_info "verification passed"
}
