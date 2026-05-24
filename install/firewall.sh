configure_firewall() {
  command -v ufw >/dev/null 2>&1 || return 0

  if sudo ufw status | grep -q 'Status: active'; then
    log_info "UFW already active, skipping configuration"
    return 0
  fi

  log_info "Configuring UFW with desktop-friendly defaults"

  sudo ufw default deny incoming
  sudo ufw default allow outgoing

  if command -v sshd >/dev/null 2>&1 || systemctl list-unit-files sshd.service >/dev/null 2>&1; then
    sudo ufw allow from 192.168.0.0/16 to any port 22 proto tcp comment 'SSH from LAN'
  fi

  sudo ufw --force enable
}
