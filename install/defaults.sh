deploy_defaults() {
  local relative_path
  local source_path
  local destination_path
  local copied_count=0

  mkdir -p "$HYPRBOLE_PATH" "$HYPRBOLE_CONFIG_PATH" "$HOME/.local/bin" "$HOME/Pictures/Wallpaper"

  rm -rf "$HYPRBOLE_PATH/default" "$HYPRBOLE_PATH/themes" "$HYPRBOLE_PATH/bin" "$HYPRBOLE_PATH/config" "$HYPRBOLE_PATH/migrations"
  cp -a "$HYPRBOLE_REPO_ROOT/default" "$HYPRBOLE_REPO_ROOT/themes" "$HYPRBOLE_REPO_ROOT/bin" "$HYPRBOLE_REPO_ROOT/config" "$HYPRBOLE_REPO_ROOT/migrations" "$HYPRBOLE_PATH/"
  chmod +x "$HYPRBOLE_PATH/bin"/* "$HYPRBOLE_PATH/default/waybar"/*.sh 2>/dev/null || true

  while IFS= read -r -d '' source_path; do
    relative_path=${source_path#"$HYPRBOLE_REPO_ROOT/config/"}
    destination_path="$HOME/.config/$relative_path"
    if copy_if_missing "$source_path" "$destination_path"; then
      copied_count=$((copied_count + 1))
    fi
  done < <(find "$HYPRBOLE_REPO_ROOT/config" -type f -print0)

  while IFS= read -r -d '' source_path; do
    ln -sf "$source_path" "$HOME/.local/bin/$(basename "$source_path")"
  done < <(find "$HYPRBOLE_PATH/bin" -maxdepth 1 -type f -print0)

  mkdir -p "$HOME/.local/share/nautilus-python/extensions"
  if [[ -f $HYPRBOLE_PATH/config/nautilus-python/extensions/hyprbole_vscode.py ]]; then
    ln -sf "$HYPRBOLE_PATH/config/nautilus-python/extensions/hyprbole_vscode.py" \
      "$HOME/.local/share/nautilus-python/extensions/hyprbole_vscode.py"
  fi

  sync_theme_assets retro-82

  mkdir -p "$HYPRBOLE_PATH/metadata"
  date --iso-8601=seconds >"$HYPRBOLE_PATH/metadata/installed-at"

  log_info "copied $copied_count user config file(s) that were missing"
}

sync_theme_assets() {
  local theme_name="$1"
  "$HYPRBOLE_PATH/bin/hyprbole" theme set "$theme_name"
}
