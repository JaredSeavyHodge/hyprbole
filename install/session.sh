configure_session() {
  local env_file="$HOME/.config/environment.d/hyprbole.conf"

  mkdir -p "$(dirname "$env_file")"
  cat >"$env_file" <<EOF
XDG_CURRENT_DESKTOP=Hyprland
XDG_SESSION_DESKTOP=Hyprland
QT_QPA_PLATFORM=wayland;xcb
QT_QPA_PLATFORMTHEME=qt6ct
GDK_BACKEND=wayland,x11
SDL_VIDEODRIVER=wayland
MOZ_ENABLE_WAYLAND=1
TERMINAL=ghostty
TERM_PROGRAM=ghostty
HYPRBOLE_PATH=$HYPRBOLE_PATH
PATH=$HOME/.local/bin:$HYPRBOLE_PATH/bin:/usr/local/sbin:/usr/local/bin:/usr/bin
EOF

  xdg-mime default org.gnome.Nautilus.desktop inode/directory || true
  xdg-mime default com.mitchellh.ghostty.desktop x-scheme-handler/terminal || true
  "$HYPRBOLE_PATH/bin/hyprbole-refresh-browser-launchers" >/dev/null 2>&1 || true
  xdg-mime default imv.desktop image/png || true
  xdg-mime default imv.desktop image/jpeg || true
  xdg-mime default imv.desktop image/gif || true
  xdg-mime default imv.desktop image/webp || true
  xdg-mime default imv.desktop image/bmp || true
  xdg-mime default imv.desktop image/tiff || true
  xdg-mime default mpv.desktop video/mp4 || true
  xdg-mime default mpv.desktop video/x-msvideo || true
  xdg-mime default mpv.desktop video/x-matroska || true
  xdg-mime default mpv.desktop video/x-flv || true
  xdg-mime default mpv.desktop video/x-ms-wmv || true
  xdg-mime default mpv.desktop video/mpeg || true
  xdg-mime default mpv.desktop video/ogg || true
  xdg-mime default mpv.desktop video/webm || true
  xdg-mime default mpv.desktop video/quicktime || true
  xdg-mime default mpv.desktop video/3gpp || true
  xdg-mime default mpv.desktop video/3gpp2 || true
  xdg-mime default mpv.desktop video/x-ms-asf || true
  xdg-mime default mpv.desktop video/x-ogm+ogg || true
  xdg-mime default mpv.desktop video/x-theora+ogg || true
  xdg-mime default mpv.desktop application/ogg || true

  setup_shell_environment
  setup_browser_theme_policy
  setup_sddm
  setup_snapper_limine
  reload_hyprland_after_install
  cleanup_hyprland_generated_stub
}

setup_browser_theme_policy() {
  "$HYPRBOLE_PATH/bin/hyprbole-setup-browser-policy" >/dev/null 2>&1 || true
}

reload_hyprland_after_install() {
  if command -v hyprctl >/dev/null 2>&1; then
    hyprctl reload >/dev/null 2>&1 || true
  fi
}

setup_shell_environment() {
  local shell_env_dir="$HOME/.config/hyprbole/shell"
  local shell_env_file="$shell_env_dir/env.sh"

  mkdir -p "$shell_env_dir"
  cat >"$shell_env_file" <<EOF
export HYPRBOLE_PATH="${HYPRBOLE_PATH}"
export TERMINAL="ghostty"
export TERM_PROGRAM="ghostty"
hyprbole_prepend_path() {
  case ":\$PATH:" in
    *":\$1:"*) ;;
    *) PATH="\$1:\$PATH" ;;
  esac
}

hyprbole_prepend_path "\$HYPRBOLE_PATH/bin"
hyprbole_prepend_path "\$HOME/.local/bin"
export PATH
EOF

  ensure_shell_source "$HOME/.profile"
  ensure_bashrc_source

  if [[ -f $HOME/.zshrc ]]; then
    ensure_shell_source "$HOME/.zshrc"
  fi
}

ensure_bashrc_source() {
  local source_line='[ -f "${HYPRBOLE_PATH:-$HOME/.local/share/hyprbole}/default/bash/rc" ] && source "${HYPRBOLE_PATH:-$HOME/.local/share/hyprbole}/default/bash/rc"'

  if copy_if_missing "$HYPRBOLE_PATH/default/bashrc" "$HOME/.bashrc"; then
    return 0
  fi

  if ! grep -Fqx "$source_line" "$HOME/.bashrc"; then
    printf '\n%s\n' "$source_line" >>"$HOME/.bashrc"
  fi
}

ensure_shell_source() {
  local shell_rc="$1"
  local source_line='[ -f "$HOME/.config/hyprbole/shell/env.sh" ] && . "$HOME/.config/hyprbole/shell/env.sh"'

  touch "$shell_rc"

  if ! grep -Fqx "$source_line" "$shell_rc"; then
    printf '\n%s\n' "$source_line" >>"$shell_rc"
  fi
}

setup_sddm() {
  if ! command -v sddm >/dev/null 2>&1; then
    return 0
  fi

  if [[ -d $HYPRBOLE_PATH/default/sddm/theme ]]; then
    sudo rm -rf /usr/share/sddm/themes/hyprbole
    sudo install -d /usr/share/sddm/themes/hyprbole
    sudo cp -a "$HYPRBOLE_PATH/default/sddm/theme/." /usr/share/sddm/themes/hyprbole/
  fi

  if [[ -f $HYPRBOLE_PATH/default/sddm/hyprbole.conf ]]; then
    sudo install -Dm644 "$HYPRBOLE_PATH/default/sddm/hyprbole.conf" /etc/sddm.conf.d/hyprbole.conf
  fi

  "$HYPRBOLE_PATH/bin/hyprbole-setup-secret-service" --quiet

  "$HYPRBOLE_PATH/bin/hyprbole-refresh-sddm" >/dev/null 2>&1 || true
}

setup_snapper_limine() {
  local limine_config=""

  if ! command -v snapper >/dev/null 2>&1; then
    return 0
  fi

  if ! command -v limine-update >/dev/null 2>&1; then
    return 0
  fi

  if ! sudo snapper list-configs | grep -q '^root '; then
    sudo snapper -c root create-config /
  fi

  if [[ -f $HYPRBOLE_PATH/default/snapper/root ]]; then
    sudo cp "$HYPRBOLE_PATH/default/snapper/root" /etc/snapper/configs/root
  fi

  if [[ -f /boot/EFI/arch-limine/limine.conf ]]; then
    limine_config="/boot/EFI/arch-limine/limine.conf"
  elif [[ -f /boot/EFI/BOOT/limine.conf ]]; then
    limine_config="/boot/EFI/BOOT/limine.conf"
  elif [[ -f /boot/EFI/limine/limine.conf ]]; then
    limine_config="/boot/EFI/limine/limine.conf"
  elif [[ -f /boot/limine/limine.conf ]]; then
    limine_config="/boot/limine/limine.conf"
  elif [[ -f /boot/limine.conf ]]; then
    limine_config="/boot/limine.conf"
  fi

  if [[ -f $HYPRBOLE_PATH/default/limine/default.conf ]]; then
    local cmdline

    if [[ -f /etc/kernel/cmdline ]]; then
      cmdline=$(</etc/kernel/cmdline)
    else
      cmdline=$(</proc/cmdline)
      cmdline=${cmdline#BOOT_IMAGE=* }
    fi

    local default_limine_config
    default_limine_config=$(<"$HYPRBOLE_PATH/default/limine/default.conf")
    default_limine_config=${default_limine_config//@@CMDLINE@@/$cmdline}
    printf '%s\n' "$default_limine_config" | sudo tee /etc/default/limine >/dev/null
  fi

  if [[ -n $limine_config ]] && [[ $limine_config != "/boot/limine.conf" ]]; then
    sudo rm -f "$limine_config"
  fi

  if [[ -f $HYPRBOLE_PATH/default/limine/limine.conf ]]; then
    sudo cp "$HYPRBOLE_PATH/default/limine/limine.conf" /boot/limine.conf
  fi

  if [[ -f /boot/limine.conf ]]; then
    # Remove the legacy placeholder entry from early Hyprbole builds.
    sudo sed -i '/^\/+$/ {
N
/^\/+\n[[:space:]]*comment:[[:space:]]*Hyprbole[[:space:]]*$/d
}' /boot/limine.conf
  fi

  sudo limine-update >/dev/null 2>&1 || true
  configure_limine_menu_defaults

  sudo btrfs quota disable / >/dev/null 2>&1 || true
  sudo systemctl enable --now snapper-cleanup.timer >/dev/null 2>&1 || true

  if command -v limine-snapper-sync >/dev/null 2>&1; then
    sudo systemctl enable limine-snapper-sync.service >/dev/null 2>&1 || true
    sudo limine-snapper-sync >/dev/null 2>&1 || true
  fi
}

configure_limine_menu_defaults() {
  [[ -f /boot/limine.conf ]] || return 0

  local tmp_file
  tmp_file=$(mktemp)

  sudo grep -vE '^(timeout|default_entry|remember_last_entry|interface_branding_color):' /boot/limine.conf >"$tmp_file"

  cat <<'EOF' | sudo tee /boot/limine.conf >/dev/null
timeout: 3
default_entry: 2
remember_last_entry: no
interface_branding_color: 9bb1ff

EOF

  sudo tee -a /boot/limine.conf <"$tmp_file" >/dev/null
  rm -f "$tmp_file"
}
