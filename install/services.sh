enable_services() {
  local failed=0
  local timer_user_units=("${HYPRBOLE_USER_TIMER_UNITS[@]}")
  local graphical_user_units=("${HYPRBOLE_GRAPHICAL_USER_UNITS[@]}")
  local first_login_units=("${HYPRBOLE_FIRST_LOGIN_USER_UNITS[@]}")
  local start_graphical_units=0
  local unit
  local service_source="$HYPRBOLE_PATH/default/systemd/system/hyprbole-health-check.service"
  local timer_source="$HYPRBOLE_PATH/default/systemd/system/hyprbole-health-check.timer"

  xdg-user-dirs-update

  deploy_user_units

  systemctl --user daemon-reload

  for unit in "${HYPRBOLE_CORE_USER_UNITS[@]}"; do
    if ! systemctl --user enable --now "$unit"; then
      printf 'failed to enable user service: %s\n' "$unit" >&2
      failed=$((failed + 1))
    fi
  done

  for unit in "${timer_user_units[@]}"; do
    if ! systemctl --user enable --now "$unit"; then
      printf 'failed to enable user timer: %s\n' "$unit" >&2
      failed=$((failed + 1))
    fi
  done

  if systemctl --user is-active --quiet graphical-session.target; then
    start_graphical_units=1
  fi

  for unit in "${graphical_user_units[@]}"; do
    if ! systemctl --user enable "$unit"; then
      printf 'failed to enable user service: %s\n' "$unit" >&2
      failed=$((failed + 1))
      continue
    fi

    if (( start_graphical_units == 1 )) && ! systemctl --user start "$unit"; then
      printf 'failed to start user service: %s\n' "$unit" >&2
      failed=$((failed + 1))
    fi
  done

  for unit in "${first_login_units[@]}"; do
    if ! systemctl --user enable "$unit"; then
      printf 'failed to enable user service: %s\n' "$unit" >&2
      failed=$((failed + 1))
    fi
  done

  if (( start_graphical_units == 0 )); then
    log_info "graphical user services enabled; they will start with the next Hyprland session"
  fi

  if ! sudo systemctl enable sddm.service; then
    printf 'failed to enable system service: sddm.service\n' >&2
    failed=$((failed + 1))
  fi

  if [[ -f $service_source && -f $timer_source ]]; then
    if ! sed "s|@HYPRBOLE_PATH@|$HYPRBOLE_PATH|g" "$service_source" | sudo install -Dm644 /dev/stdin /etc/systemd/system/hyprbole-health-check.service; then
      printf 'failed to install system service: hyprbole-health-check.service\n' >&2
      failed=$((failed + 1))
    fi

    if ! sudo install -Dm644 "$timer_source" /etc/systemd/system/hyprbole-health-check.timer; then
      printf 'failed to install system timer: hyprbole-health-check.timer\n' >&2
      failed=$((failed + 1))
    fi

    sudo systemctl daemon-reload

    if ! sudo systemctl enable --now hyprbole-health-check.timer; then
      printf 'failed to enable system timer: hyprbole-health-check.timer\n' >&2
      failed=$((failed + 1))
    fi
  fi

  if command -v limine-snapper-sync >/dev/null 2>&1 && ! sudo systemctl enable limine-snapper-sync.service; then
    printf 'failed to enable system service: limine-snapper-sync.service\n' >&2
    failed=$((failed + 1))
  fi

  (( failed == 0 )) || die "service setup failed with $failed issue(s)"
}

deploy_user_units() {
  local unit_dir="$HOME/.config/systemd/user"
  local source_dir="$HYPRBOLE_PATH/default/systemd/user"

  [[ -d $source_dir ]] || return 0

  mkdir -p "$unit_dir"

  while IFS= read -r -d '' unit; do
    local name
    name=$(basename "$unit")
    if [[ ! -f $unit_dir/$name ]] || ! cmp -s "$unit" "$unit_dir/$name"; then
      install -m 644 "$unit" "$unit_dir/$name"
      log_info "deployed user unit: $name"
    fi
  done < <(find "$source_dir" -type f \( -name '*.service' -o -name '*.timer' \) -print0)
}
