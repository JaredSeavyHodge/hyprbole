HYPRBOLE_AUR_PACKAGES=(
  1password-beta
  1password-cli
  brave-origin-nightly-bin
  walker-bin
  elephant
  elephant-1password
  elephant-calc
  elephant-clipboard
  elephant-desktopapplications
  elephant-files
  elephant-menus
  elephant-runner
  elephant-symbols
  elephant-websearch
  hyprsysteminfo
  limine-snapper-sync
  limine-mkinitcpio-hook
)

install_aur_packages() {
  local yay_args=(-S --needed --noconfirm --answerclean None --answerdiff None --answeredit None --answerupgrade None)

  yay "${yay_args[@]}" "${HYPRBOLE_AUR_PACKAGES[@]}"
}
