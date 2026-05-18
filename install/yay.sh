install_yay() {
  local tmp_dir

  if cmd_present yay; then
    log_info "yay already installed"
    return 0
  fi

  tmp_dir=$(mktemp -d)
  git clone https://aur.archlinux.org/yay-bin.git "$tmp_dir/yay-bin"

  (
    cd "$tmp_dir/yay-bin"
    makepkg -si --noconfirm
  )

  rm -rf "$tmp_dir"
}
