install_aur_packages() {
  local yay_args=(-S --needed --noconfirm --answerclean None --answerdiff None --answeredit None --answerupgrade None)

  if [[ -x ${HYPRBOLE_REPO_ROOT:-}/bin/hyprbole-health-check ]]; then
    "$HYPRBOLE_REPO_ROOT/bin/hyprbole-health-check" --disk-space-guard
  fi

  yay "${yay_args[@]}" "${HYPRBOLE_AUR_PACKAGES[@]}"
}
