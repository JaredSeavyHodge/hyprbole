HYPRBOLE_AUR_PACKAGES=(
  1password-beta
  1password-cli
  brave-origin-nightly-bin
  walker-bin
  elephant-bin
  elephant-clipboard-bin
  elephant-calc-bin
  elephant-desktopapplications-bin
  elephant-files-bin
  elephant-1password-bin
  elephant-menus-bin
  elephant-providerlist-bin
  elephant-runner-bin
  elephant-symbols-bin
  elephant-websearch-bin
  hyprsysteminfo
  limine-snapper-sync
  limine-mkinitcpio-hook
)

install_aur_packages() {
  local yay_args=(-S --needed --noconfirm --answerclean None --answerdiff None --answeredit None --answerupgrade None)

  yay "${yay_args[@]}" "${HYPRBOLE_AUR_PACKAGES[@]}"
}
