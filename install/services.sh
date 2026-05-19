enable_services() {
  local failed=0
  local graphical_user_units=(swayosd-server.service polkit-gnome-agent.service elephant.service walker.service swaync.service)
  local start_graphical_units=0
  local unit

  xdg-user-dirs-update

  systemctl --user daemon-reload

  for unit in pipewire.service pipewire-pulse.service wireplumber.service gnome-keyring-daemon.socket; do
    if ! systemctl --user enable --now "$unit"; then
      printf 'failed to enable user service: %s\n' "$unit" >&2
      failed=$((failed + 1))
    fi
  done

  if systemctl --user is-active --quiet graphical-session.target; then
    start_graphical_units=1
  fi

  for unit in "${graphical_user_units[@]}"; do
    if ! systemctl --user enable "$unit"; then
      printf 'failed to enable user service: %s\n' "$unit" >&2
      failed=$((failed + 1))
      continue
    fi

    if (( start_graphical_units == 1 )) && ! systemctl --user start "$unit"; then
      printf 'failed to start user service: %s\n' "$unit" >&2
      failed=$((failed + 1))
    fi
  done

  if (( start_graphical_units == 0 )); then
    log_info "graphical user services enabled; they will start with the next Hyprland session"
  fi

  if ! sudo systemctl enable sddm.service; then
    printf 'failed to enable system service: sddm.service\n' >&2
    failed=$((failed + 1))
  fi

  if command -v limine-snapper-sync >/dev/null 2>&1 && ! sudo systemctl enable limine-snapper-sync.service; then
    printf 'failed to enable system service: limine-snapper-sync.service\n' >&2
    failed=$((failed + 1))
  fi

  (( failed == 0 )) || die "service setup failed with $failed issue(s)"
}
