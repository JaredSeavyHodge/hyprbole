HYPRBOLE_OFFICIAL_PACKAGES=(
  hyprland
  uwsm
  waybar
  sddm
  swaync
  swayosd
  pipewire
  pipewire-audio
  pipewire-alsa
  pipewire-pulse
  wireplumber
  xdg-desktop-portal
  xdg-desktop-portal-hyprland
  xdg-desktop-portal-gtk
  polkit-gnome
  qt5-wayland
  qt6-wayland
  hypridle
  hyprlock
  awww
  wf-recorder
  nautilus
  nautilus-python
  imv
  mpv
  ghostty
  btop
  bash-completion
  mise
  starship
  zoxide
  fzf
  bat
  fd
  ripgrep
  unzip
  lazygit
  tree-sitter-cli
  obsidian
  opencode
  neovim
  polkit
  wl-clipboard
  grim
  slurp
  satty
  pavucontrol
  nm-connection-editor
  network-manager-applet
  blueman
  gnome-keyring
  playerctl
  brightnessctl
  xdg-user-dirs
  xdg-utils
  gvfs
  gvfs-mtp
  libsecret
  git
  curl
  jq
  imagemagick
  libqalculate
  pacman-contrib
  limine
  gnome-themes-extra
  noto-fonts
  noto-fonts-cjk
  noto-fonts-emoji
  noto-fonts-extra
  rust
  ttf-cascadia-code
  ttf-dejavu
  ttf-fira-code
  ttf-hack
  ttf-jetbrains-mono
  ttf-jetbrains-mono-nerd
  ttf-liberation
  ttf-nerd-fonts-symbols
  ttf-opensans
  ttf-roboto
  ttf-ubuntu-font-family
  inter-font
  otf-font-awesome
  papirus-icon-theme
  adwaita-icon-theme
  hicolor-icon-theme
  base-devel
  snapper
)

HYPRBOLE_CONFLICTING_PACKAGES=(
  dunst
  pulseaudio
  pulseaudio-alsa
  pulseaudio-bluetooth
)

HYPRBOLE_PROFILE_LEFTOVER_PACKAGES=(
  dolphin
  kitty
  polkit-kde-agent
  wofi
)

hyprbole_installed_packages() {
  local package

  for package in "$@"; do
    if pacman -Q "$package" >/dev/null 2>&1; then
      printf '%s\n' "$package"
    fi
  done
}

remove_conflicting_packages() {
  local installed_packages=()
  local package

  while IFS= read -r package; do
    [[ -n $package ]] || continue
    installed_packages+=("$package")
  done < <(hyprbole_installed_packages "${HYPRBOLE_CONFLICTING_PACKAGES[@]}")

  if ((${#installed_packages[@]} == 0)); then
    log_info "no conflicting packages found"
    return 0
  fi

  log_info "removing conflicting package(s): ${installed_packages[*]}"
  sudo pacman -Rns --noconfirm "${installed_packages[@]}"
}

warn_profile_leftovers() {
  local installed_packages=()
  local package

  while IFS= read -r package; do
    [[ -n $package ]] || continue
    installed_packages+=("$package")
  done < <(hyprbole_installed_packages "${HYPRBOLE_PROFILE_LEFTOVER_PACKAGES[@]}")

  if ((${#installed_packages[@]} == 0)); then
    return 0
  fi

  log_info "profile leftover package(s) still installed: ${installed_packages[*]}"
  log_info "Hyprbole does not require these; remove them manually if you do not use them"
}

install_official_packages() {
  local pacman_args=(-Syu --needed --noconfirm)

  sudo pacman "${pacman_args[@]}" "${HYPRBOLE_OFFICIAL_PACKAGES[@]}"
}
