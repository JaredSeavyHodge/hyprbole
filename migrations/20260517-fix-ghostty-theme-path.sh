echo "Update Ghostty to load Hyprbole current theme config from ~/.config"

ghostty_config="$HOME/.config/ghostty/config"

if [[ -f $ghostty_config ]]; then
  sed -i 's|~/.local/share/hyprbole/current/theme/ghostty.conf|~/.config/hyprbole/current/theme/ghostty.conf|g' "$ghostty_config"
fi
