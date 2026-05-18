enable_services() {
  xdg-user-dirs-update

  systemctl --user daemon-reload || true

  for unit in pipewire.service pipewire-pulse.service wireplumber.service swayosd-server.service polkit-gnome-agent.service elephant.service walker.service; do
    systemctl --user enable --now "$unit" >/dev/null 2>&1 || true
  done

  sudo systemctl enable sddm.service >/dev/null 2>&1 || true
  sudo systemctl enable limine-snapper-sync.service >/dev/null 2>&1 || true
}
