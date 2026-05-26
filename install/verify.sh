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

verify_file_current() {
  local source="$1"
  local target="$2"
  local label="$3"

  if [[ ! -r $source || ! -e $target ]]; then
    printf 'missing %s file: %s\n' "$label" "$target" >&2
    return 1
  fi

  if [[ ! -r $target ]]; then
    if sudo -n true >/dev/null 2>&1; then
      if ! sudo cmp -s "$source" "$target"; then
        printf '%s differs from Hyprbole default: %s\n' "$label" "$target" >&2
        return 1
      fi
    else
      printf 'warning: cannot compare unreadable %s file without sudo: %s\n' "$label" "$target" >&2
    fi
    return 0
  fi

  if ! cmp -s "$source" "$target"; then
    printf '%s differs from Hyprbole default: %s\n' "$label" "$target" >&2
    return 1
  fi
}

verify_default_limine_config() {
  local source="$HYPRBOLE_PATH/default/limine/default.conf"
  local target="/etc/default/limine"
  local cmdline
  local expected
  local tmp_file

  [[ -r $source && -e $target ]] || {
    printf 'missing Limine default config: %s\n' "$target" >&2
    return 1
  }

  if [[ -f /etc/kernel/cmdline ]]; then
    cmdline=$(</etc/kernel/cmdline)
  else
    cmdline=$(</proc/cmdline)
    cmdline=${cmdline#BOOT_IMAGE=* }
  fi

  expected=$(<"$source")
  expected=${expected//@@CMDLINE@@/$cmdline}
  tmp_file=$(mktemp)
  printf '%s\n' "$expected" >"$tmp_file"

  if [[ ! -r $target ]]; then
    if sudo -n true >/dev/null 2>&1; then
      if ! sudo cmp -s "$tmp_file" "$target"; then
        rm -f "$tmp_file"
        printf 'Limine default config differs from Hyprbole default: %s\n' "$target" >&2
        return 1
      fi
    else
      printf 'warning: cannot compare unreadable Limine default config without sudo: %s\n' "$target" >&2
    fi
    rm -f "$tmp_file"
    return 0
  fi

  if ! cmp -s "$tmp_file" "$target"; then
    rm -f "$tmp_file"
    printf 'Limine default config differs from Hyprbole default: %s\n' "$target" >&2
    return 1
  fi

  rm -f "$tmp_file"
}

verify_limine_menu_defaults() {
  local config="/boot/limine.conf"
  local actual
  local default_count
  local expected_lines=(
    'timeout: 3'
    'default_entry: 2'
    'remember_last_entry: no'
    'interface_branding_color: 9bb1ff'
  )
  local i
  local line
  local use_sudo=0

  [[ -e $config ]] || {
    printf 'missing Limine menu config: %s\n' "$config" >&2
    return 1
  }

  if [[ ! -r $config ]]; then
    if sudo -n true >/dev/null 2>&1; then
      use_sudo=1
    else
      printf 'warning: cannot compare unreadable Limine menu config without sudo: %s\n' "$config" >&2
      return 0
    fi
  fi

  for i in "${!expected_lines[@]}"; do
    line="${expected_lines[$i]}"
    if (( use_sudo == 1 )); then
      actual="$(sudo awk -v line_number="$((i + 1))" 'NR == line_number { print; exit }' "$config")"
    else
      actual="$(awk -v line_number="$((i + 1))" 'NR == line_number { print; exit }' "$config")"
    fi

    if [[ $actual != "$line" ]]; then
      printf 'Limine menu config differs from Hyprbole default at line %s: %s\n' "$((i + 1))" "$config" >&2
      return 1
    fi
  done

  if (( use_sudo == 1 )); then
    default_count="$(sudo grep -Ec '^(timeout|default_entry|remember_last_entry|interface_branding_color):' "$config" || printf '0')"
  else
    default_count="$(grep -Ec '^(timeout|default_entry|remember_last_entry|interface_branding_color):' "$config" || printf '0')"
  fi

  if [[ $default_count != 4 ]]; then
    printf 'Limine menu config has duplicate or missing Hyprbole defaults: %s\n' "$config" >&2
    return 1
  fi
}

verify_installation() {
  local failures=0
  local binary
  local failed_units
  local graphical_session_active=0
  local package
  local policy_target
  local process_label
  local secret_service_failures=0
  local unit
  local active_user_units=("${HYPRBOLE_ACTIVE_USER_UNITS[@]}")
  local graphical_user_units=("${HYPRBOLE_GRAPHICAL_USER_UNITS[@]}")
  local enabled_user_units=("${HYPRBOLE_ENABLED_USER_UNITS[@]}")
  local user_timer_units=("${HYPRBOLE_USER_TIMER_UNITS[@]}")

  if systemctl --user is-active --quiet graphical-session.target; then
    graphical_session_active=1
  fi

  for binary in "${HYPRBOLE_VERIFY_REQUIRED_COMMANDS[@]}"; do
    if ! cmd_present "$binary"; then
      printf 'missing command: %s\n' "$binary" >&2
      failures=$((failures + 1))
    fi
  done

  for package in "${HYPRBOLE_VERIFY_REQUIRED_PACKAGES[@]}"; do
    if ! pacman -Q "$package" >/dev/null 2>&1; then
      printf 'missing package: %s\n' "$package" >&2
      failures=$((failures + 1))
    fi
  done

  for path in "${HYPRBOLE_VERIFY_REQUIRED_PATHS[@]}"; do
    if [[ $path == */bin/hyprbole-lib.d/* ]]; then
      if [[ ! -f $path || ! -r $path ]]; then
        printf 'missing readable helper module: %s\n' "$path" >&2
        failures=$((failures + 1))
      fi
    elif [[ ! -e $path ]]; then
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

  for path in "${HYPRBOLE_BROWSER_POLICY_SYMLINK_PATHS[@]}"; do
    policy_target=""

    if [[ ! -L $path ]]; then
      printf 'browser policy should be a symlink: %s\n' "$path" >&2
      failures=$((failures + 1))
    else
      policy_target="$(readlink -f "$path" 2>/dev/null || true)"
    fi

    if [[ -L $path && ! -f $HYPRBOLE_CONFIG_PATH/current/browser-policy.json ]]; then
      printf 'browser policy target is missing: %s\n' "$HYPRBOLE_CONFIG_PATH/current/browser-policy.json" >&2
      failures=$((failures + 1))
    elif [[ -L $path && $policy_target != "$HYPRBOLE_CONFIG_PATH/current/browser-policy.json" ]]; then
      printf 'browser policy symlink should target Hyprbole policy state: %s\n' "$path" >&2
      failures=$((failures + 1))
    fi
  done

  verify_file_current "$HYPRBOLE_PATH/default/snapper/root" /etc/snapper/configs/root "Snapper root config" || failures=$((failures + 1))
  verify_default_limine_config || failures=$((failures + 1))
  verify_limine_menu_defaults || failures=$((failures + 1))

  if command -v code >/dev/null 2>&1 && ! grep -Fxq -- '--password-store=gnome-libsecret' "$HOME/.config/code-flags.conf"; then
    printf 'VS Code should use gnome-libsecret password store: %s\n' "$HOME/.config/code-flags.conf" >&2
    failures=$((failures + 1))
  fi

  if ! grep -Fxq -- '--password-store=gnome-libsecret' "$HOME/.config/brave-origin-nightly-flags.conf"; then
    printf 'Brave flags should use gnome-libsecret password store: %s\n' "$HOME/.config/brave-origin-nightly-flags.conf" >&2
    failures=$((failures + 1))
  fi

  if ! grep -Fxq -- '--ozone-platform-hint=auto' "$HOME/.config/brave-origin-nightly-flags.conf"; then
    printf 'Brave flags should set --ozone-platform-hint=auto: %s\n' "$HOME/.config/brave-origin-nightly-flags.conf" >&2
    failures=$((failures + 1))
  fi

  if [[ ! -x $HYPRBOLE_PATH/bin/hyprbole-launch-brave-origin-nightly ]]; then
    printf 'missing executable Brave launcher: %s\n' "$HYPRBOLE_PATH/bin/hyprbole-launch-brave-origin-nightly" >&2
    failures=$((failures + 1))
  fi

  if [[ -f $HOME/.local/share/applications/hyprbole-disk-usage.desktop ]] && ! grep -Fxq "Exec=$HYPRBOLE_PATH/bin/hyprbole launch disk-usage" "$HOME/.local/share/applications/hyprbole-disk-usage.desktop"; then
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

  for unit in "${user_timer_units[@]}"; do
    if ! systemctl --user is-enabled "$unit" >/dev/null 2>&1; then
      printf 'user timer is not enabled: %s\n' "$unit" >&2
      failures=$((failures + 1))
    elif ! systemctl --user is-active "$unit" >/dev/null 2>&1; then
      printf 'user timer is not active: %s\n' "$unit" >&2
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
    printf 'system timer is not enabled: hyprbole-health-check.timer\n' >&2
    failures=$((failures + 1))
  fi

  if command -v limine-snapper-sync >/dev/null 2>&1 && ! systemctl is-enabled limine-snapper-sync.service >/dev/null 2>&1; then
    printf 'system service is not enabled: limine-snapper-sync.service\n' >&2
    failures=$((failures + 1))
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
