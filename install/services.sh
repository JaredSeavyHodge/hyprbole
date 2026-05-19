enable_services() {
  local failed=0
  local unit

  xdg-user-dirs-update

  systemctl --user daemon-reload

  for unit in pipewire.service pipewire-pulse.service wireplumber.service swayosd-server.service polkit-gnome-agent.service gnome-keyring-daemon.socket elephant.service walker.service swaync.service; do
    if ! systemctl --user enable --now "$unit"; then
      printf 'failed to enable user service: %s\n' "$unit" >&2
      failed=$((failed + 1))
    fi
  done

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
