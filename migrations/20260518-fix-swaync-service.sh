echo "Update SwayNC user service to avoid notification D-Bus conflicts"

swaync_service="$HOME/.config/systemd/user/swaync.service"
source_service="${HYPRBOLE_PATH:-$HOME/.local/share/hyprbole}/config/systemd/user/swaync.service"

if [[ -f $swaync_service ]] && [[ -f $source_service ]]; then
  if grep -Fxq 'Type=dbus' "$swaync_service" && grep -Fxq 'BusName=org.freedesktop.Notifications' "$swaync_service"; then
    cp "$swaync_service" "$swaync_service.bak.$(date +%Y%m%d%H%M%S)"
    cp "$source_service" "$swaync_service"
    systemctl --user daemon-reload >/dev/null 2>&1 || true
    systemctl --user restart swaync.service >/dev/null 2>&1 || true
  fi
fi
