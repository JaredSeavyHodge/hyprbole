verify_runtime_process() {
  local process="$1"
  local label="$2"
  local attempt

  for attempt in 1 2 3; do
    if pgrep -x "$process" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.2
  done

  printf 'graphical runtime process is not running: %s (%s)\n' "$label" "$process" >&2
  return 1
}

verify_pam_hook() {
  local path="$1"
  local label="$2"
  local type="$3"
  local expectation="${4:-present}"
  local pattern="^[[:space:]]*-?${type}[[:space:]]+.*pam_gnome_keyring\\.so"
  local present=0

  if [[ ! -r $path ]]; then
    printf 'missing PAM file: %s\n' "$path" >&2
    return 1
  fi

  if grep -Eq "$pattern" "$path"; then
    present=1
  fi

  case "$expectation" in
    present)
      if (( present == 1 )); then
        return 0
      fi
      printf 'missing PAM hook: %s\n' "$label" >&2
      ;;
    absent)
      if (( present == 0 )); then
        return 0
      fi
      printf 'PAM hook should be removed: %s\n' "$label" >&2
      ;;
    *)
      printf 'unknown PAM hook expectation: %s\n' "$expectation" >&2
      ;;
  esac

  return 1
}

verify_secret_service() {
  local failures=0
  local default_file="$HOME/.local/share/keyrings/default"
  local default_keyring="$HOME/.local/share/keyrings/Default_keyring.keyring"
  local login_keyring="$HOME/.local/share/keyrings/login.keyring"
  local output

  if [[ ! -f $default_keyring ]]; then
    printf 'missing default Secret Service keyring: %s\n' "$default_keyring" >&2
    failures=$((failures + 1))
  fi

  if [[ ! -f $default_file ]] || ! grep -Fxq 'Default_keyring' "$default_file"; then
    printf 'Secret Service default keyring should be Default_keyring: %s\n' "$default_file" >&2
    failures=$((failures + 1))
  fi

  if [[ -f $login_keyring ]]; then
    printf 'encrypted login keyring remains and can shadow Default_keyring: %s\n' "$login_keyring" >&2
    failures=$((failures + 1))
  fi

  verify_pam_hook /etc/pam.d/sddm "sddm auth pam_gnome_keyring" auth absent || failures=$((failures + 1))
  verify_pam_hook /etc/pam.d/sddm "sddm password pam_gnome_keyring" password absent || failures=$((failures + 1))
  verify_pam_hook /etc/pam.d/sddm "sddm session pam_gnome_keyring" session present || failures=$((failures + 1))
  verify_pam_hook /etc/pam.d/sddm-autologin "sddm-autologin session pam_gnome_keyring" session present || failures=$((failures + 1))

  if ! systemctl --user is-enabled gnome-keyring-daemon.socket >/dev/null 2>&1; then
    printf 'user unit is not enabled: gnome-keyring-daemon.socket\n' >&2
    failures=$((failures + 1))
  fi

  if command -v busctl >/dev/null 2>&1; then
    if timeout 3 busctl --user --no-pager status org.freedesktop.secrets >/dev/null 2>&1; then
      if ! timeout 3 busctl --user call org.freedesktop.secrets /org/freedesktop/secrets org.freedesktop.Secret.Service OpenSession sv plain s '' >/dev/null 2>&1; then
        printf 'Secret Service session did not open\n' >&2
        failures=$((failures + 1))
      fi

      output="$(timeout 3 busctl --user call org.freedesktop.secrets /org/freedesktop/secrets org.freedesktop.Secret.Service ReadAlias s default 2>/dev/null || true)"
      if [[ $output == 'o "/org/freedesktop/secrets/collection/login"' ]]; then
        printf 'Secret Service default alias still points at login collection\n' >&2
        failures=$((failures + 1))
      elif [[ $output != o\ \"/org/freedesktop/secrets/collection/* ]]; then
        printf 'Secret Service default alias is not readable\n' >&2
        failures=$((failures + 1))
      else
        local collection_path="${output#o \"}"
        collection_path="${collection_path%\"}"

        if ! timeout 3 busctl --user get-property org.freedesktop.secrets "$collection_path" org.freedesktop.Secret.Collection Locked >/dev/null 2>&1; then
          printf 'Secret Service default collection is not available: %s\n' "$collection_path" >&2
          failures=$((failures + 1))
        fi
      fi
    else
      printf 'warning: org.freedesktop.secrets is not owned in this session\n' >&2
    fi
  fi

  return "$failures"
}

verify_installation() {
  local failures=0
  local binary
  local failed_units
  local graphical_session_active=0
  local process_label
  local secret_service_failures=0
  local unit
  local active_user_units=(pipewire.service pipewire-pulse.service wireplumber.service)
  local graphical_user_units=(swayosd-server.service polkit-gnome-agent.service elephant.service walker.service swaync.service)
  local first_login_units=(hyprbole-first-login-guide.service)
  local enabled_user_units=(gnome-keyring-daemon.socket "${graphical_user_units[@]}" "${first_login_units[@]}")

  if systemctl --user is-active --quiet graphical-session.target; then
    graphical_session_active=1
  fi

  for binary in hyprland uwsm waybar ghostty nautilus swaync swayosd-client yay snapper sddm limine-update code nvim rg fd eza dua unzip lazygit tree-sitter man nvme smartctl; do
    if ! cmd_present "$binary"; then
      printf 'missing command: %s\n' "$binary" >&2
      failures=$((failures + 1))
    fi
  done

  for path in \
    "$HYPRBOLE_PATH/bin/hyprbole-health-check" \
    "$HYPRBOLE_PATH/bin/hyprbole-health-indicator" \
    "$HYPRBOLE_PATH/bin/hyprbole-health-report" \
    "$HYPRBOLE_PATH/bin/hyprbole-health-updates" \
    "$HYPRBOLE_PATH/bin/hyprbole-update-indicator" \
    "$HYPRBOLE_PATH/bin/hyprbole-first-login-guide" \
    "$HOME/.config/hypr/hyprland.lua" \
    "$HOME/.config/systemd/user/hyprbole-first-login-guide.service" \
    "$HYPRBOLE_CONFIG_PATH/theme-sources.conf" \
    "$HOME/.config/waybar/config.jsonc" \
    "$HYPRBOLE_PATH/default/systemd/system/hyprbole-health-check.service" \
    "$HYPRBOLE_PATH/default/systemd/system/hyprbole-health-check.timer" \
    "/etc/systemd/system/hyprbole-health-check.service" \
    "/etc/systemd/system/hyprbole-health-check.timer" \
    "/etc/sddm.conf.d/hyprbole.conf" \
    "/usr/share/sddm/themes/hyprbole/Main.qml" \
    "/usr/share/sddm/themes/hyprbole/metadata.desktop" \
    "/usr/share/sddm/themes/hyprbole/theme.conf" \
    "/usr/share/sddm/themes/hyprbole/background.jpg" \
    "/usr/share/sddm/themes/hyprbole/background-blur.jpg" \
    "$HYPRBOLE_PATH/assets/logo/circle-hyprbole-logo.svg" \
    "$HOME/.config/xdg-desktop-portal/hyprland-portals.conf" \
    "$HOME/.config/gtk-3.0/settings.ini" \
    "$HOME/.config/gtk-4.0/settings.ini" \
    "$HOME/.config/code-flags.conf" \
    "$HOME/.config/brave-origin-nightly-flags.conf" \
    "$HOME/.local/share/applications/hyprbole-brave-origin-nightly.desktop" \
    "$HOME/.local/share/applications/hyprbole-disk-usage.desktop" \
    "$HOME/.config/elephant/menus/hyprbole-tools.toml" \
    "$HOME/.config/elephant/menus/hyprbole-fonts.lua" \
    "$HOME/.config/elephant/menus/hyprbole-remove.toml" \
    "$HOME/.config/elephant/menus/hyprbole-power-profiles.toml" \
    "$HOME/.config/nvim/init.lua" \
    "$HOME/.config/nvim/lua/config/lazy.lua" \
    "$HOME/.config/nvim/lua/plugins/hyprbole-theme.lua" \
    "$HYPRBOLE_CONFIG_PATH/current/theme/neovim.lua" \
    "$HYPRBOLE_CONFIG_PATH/current/theme/vscode.json" \
    "$HYPRBOLE_PATH/default" \
    "$HYPRBOLE_PATH/themes"; do
    if [[ ! -e $path ]]; then
      printf 'missing path: %s\n' "$path" >&2
      failures=$((failures + 1))
    fi
  done

  if ! git_checkout "$HYPRBOLE_PATH"; then
    printf 'HYPRBOLE_PATH is not a git checkout root: %s\n' "$HYPRBOLE_PATH" >&2
    failures=$((failures + 1))
  fi

  if [[ -f /usr/share/sddm/themes/hyprbole/Main.qml ]] && ! cmp -s "$HYPRBOLE_PATH/default/sddm/theme/Main.qml" /usr/share/sddm/themes/hyprbole/Main.qml; then
    printf 'SDDM Main.qml differs from Hyprbole default: %s\n' "/usr/share/sddm/themes/hyprbole/Main.qml" >&2
    failures=$((failures + 1))
  fi

  if [[ -f /usr/share/sddm/themes/hyprbole/metadata.desktop ]] && ! cmp -s "$HYPRBOLE_PATH/default/sddm/theme/metadata.desktop" /usr/share/sddm/themes/hyprbole/metadata.desktop; then
    printf 'SDDM metadata.desktop differs from Hyprbole default: %s\n' "/usr/share/sddm/themes/hyprbole/metadata.desktop" >&2
    failures=$((failures + 1))
  fi

  if [[ -f /etc/sddm.conf.d/hyprbole.conf ]] && ! grep -Fxq 'Current=hyprbole' /etc/sddm.conf.d/hyprbole.conf; then
    printf 'SDDM theme config should select hyprbole: %s\n' "/etc/sddm.conf.d/hyprbole.conf" >&2
    failures=$((failures + 1))
  fi

  if [[ -f $HOME/.config/hypr/hyprland.conf ]]; then
    if grep -Fq 'autogenerated = 1' "$HOME/.config/hypr/hyprland.conf" && grep -Fq 'This config is a STUB' "$HOME/.config/hypr/hyprland.conf"; then
      printf 'warning: autogenerated Hyprland stub remains: %s\n' "$HOME/.config/hypr/hyprland.conf" >&2
    else
      printf 'conflicting Hyprland config remains: %s\n' "$HOME/.config/hypr/hyprland.conf" >&2
      printf 'Hyprbole uses hyprland.lua; move or remove hyprland.conf if it is not intentional.\n' >&2
      failures=$((failures + 1))
    fi
  fi

  for path in \
    /etc/brave/policies/managed/color.json; do
    if [[ -e $path && ! -L $path ]]; then
      printf 'browser policy should be a symlink: %s\n' "$path" >&2
      failures=$((failures + 1))
    fi
  done

  if ! grep -Fxq -- '--password-store=gnome-libsecret' "$HOME/.config/code-flags.conf"; then
    printf 'VS Code should use gnome-libsecret password store: %s\n' "$HOME/.config/code-flags.conf" >&2
    failures=$((failures + 1))
  fi

  if ! grep -Fxq -- '--password-store=gnome-libsecret' "$HOME/.config/brave-origin-nightly-flags.conf"; then
    printf 'Brave fallback wrapper should use gnome-libsecret password store: %s\n' "$HOME/.config/brave-origin-nightly-flags.conf" >&2
    failures=$((failures + 1))
  fi

  if [[ ! -x $HYPRBOLE_PATH/bin/hyprbole-launch-brave-origin-nightly ]]; then
    printf 'missing executable Brave launcher: %s\n' "$HYPRBOLE_PATH/bin/hyprbole-launch-brave-origin-nightly" >&2
    failures=$((failures + 1))
  else
    if ! grep -Fxq -- '  --password-store=gnome-libsecret' "$HYPRBOLE_PATH/bin/hyprbole-launch-brave-origin-nightly"; then
      printf 'Hyprbole Brave launcher should use gnome-libsecret password store\n' >&2
      failures=$((failures + 1))
    fi

    if ! grep -Fxq -- '  --ozone-platform-hint=auto' "$HYPRBOLE_PATH/bin/hyprbole-launch-brave-origin-nightly"; then
      printf 'Hyprbole Brave launcher should use automatic Wayland/X11 ozone selection\n' >&2
      failures=$((failures + 1))
    fi
  fi

  if [[ -f $HOME/.local/share/applications/hyprbole-brave-origin-nightly.desktop ]] && ! grep -Fxq "Exec=$HYPRBOLE_PATH/bin/hyprbole-launch-brave-origin-nightly %U" "$HOME/.local/share/applications/hyprbole-brave-origin-nightly.desktop"; then
    printf 'Brave desktop launcher should use Hyprbole wrapper: %s\n' "$HOME/.local/share/applications/hyprbole-brave-origin-nightly.desktop" >&2
    failures=$((failures + 1))
  fi

  if [[ -f $HOME/.local/share/applications/hyprbole-disk-usage.desktop ]] && ! grep -Fxq "Exec=$HYPRBOLE_PATH/bin/hyprbole-launch-disk-usage" "$HOME/.local/share/applications/hyprbole-disk-usage.desktop"; then
    printf 'Disk Usage desktop launcher should use Hyprbole wrapper: %s\n' "$HOME/.local/share/applications/hyprbole-disk-usage.desktop" >&2
    failures=$((failures + 1))
  fi

  verify_secret_service || secret_service_failures=$?
  failures=$((failures + secret_service_failures))

  for unit in "${active_user_units[@]}"; do
    if ! systemctl --user is-enabled "$unit" >/dev/null 2>&1; then
      printf 'user service is not enabled: %s\n' "$unit" >&2
      failures=$((failures + 1))
    elif ! systemctl --user is-active "$unit" >/dev/null 2>&1; then
      printf 'user service is not active: %s\n' "$unit" >&2
      failures=$((failures + 1))
    fi
  done

  for unit in "${enabled_user_units[@]}"; do
    if ! systemctl --user is-enabled "$unit" >/dev/null 2>&1; then
      printf 'user unit is not enabled: %s\n' "$unit" >&2
      failures=$((failures + 1))
    fi
  done

  if (( graphical_session_active == 1 )); then
    for unit in "${graphical_user_units[@]}"; do
      if ! systemctl --user is-active "$unit" >/dev/null 2>&1; then
        printf 'graphical user service is not active: %s\n' "$unit" >&2
        failures=$((failures + 1))
      fi
    done

    for process_label in \
      waybar:Waybar \
      swaync:SwayNC \
      swayosd-server:SwayOSD \
      walker:Walker \
      elephant:Elephant; do
      if ! verify_runtime_process "${process_label%%:*}" "${process_label#*:}"; then
        failures=$((failures + 1))
      fi
    done

    if [[ -f $HYPRBOLE_CONFIG_PATH/current/background ]] && ! verify_runtime_process awww-daemon "wallpaper daemon"; then
      failures=$((failures + 1))
    fi
  fi

  if command -v snapper >/dev/null 2>&1; then
    if ! systemctl is-enabled snapper-cleanup.timer >/dev/null 2>&1; then
      printf 'warning: system timer is not enabled: snapper-cleanup.timer\n' >&2
    fi
  fi

  if ! systemctl is-enabled hyprbole-health-check.timer >/dev/null 2>&1; then
    printf 'warning: system timer is not enabled: hyprbole-health-check.timer\n' >&2
  fi

  failed_units="$(systemctl --user list-units --state=failed --no-legend --plain 2>/dev/null || true)"
  if [[ -n $failed_units ]]; then
    if (( graphical_session_active == 1 )); then
      printf 'failed user services remain:\n' >&2
      printf '%s\n' "$failed_units" >&2
      failures=$((failures + 1))
    else
      printf 'warning: failed user services are present before graphical login:\n' >&2
      printf '%s\n' "$failed_units" >&2
    fi
  fi

  if (( failures > 0 )); then
    die "verification failed with $failures issue(s)"
  fi

  log_info "verification passed"
}
