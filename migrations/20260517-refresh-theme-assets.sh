echo "Refresh current theme assets if a theme is already selected"

if [[ -f "$HOME/.config/hyprbole/current/theme-name" ]]; then
  theme_name=$(cat "$HOME/.config/hyprbole/current/theme-name")
  if [[ -n $theme_name ]] && command -v hyprbole >/dev/null 2>&1; then
    hyprbole theme set "$theme_name" >/dev/null 2>&1 || true
  fi
fi
