#!/bin/bash

set -euo pipefail

bell_icon=""
muted_icon="󰂛"

json_escape() {
  jq -Rn --arg value "$1" '$value'
}

emit() {
  local text="$1"
  local class_name="$2"
  local tooltip="$3"

  printf '{"text":%s,"class":%s,"tooltip":%s}\n' \
    "$(json_escape "$text")" \
    "$(json_escape "$class_name")" \
    "$(json_escape "$tooltip")"
}

if ! command -v swaync-client >/dev/null 2>&1; then
  emit "" hidden "SwayNC is not installed"
  exit 0
fi

if ! command -v jq >/dev/null 2>&1; then
  printf '{"text":"%s","class":"warning","tooltip":"Notification indicator missing jq"}\n' "$bell_icon"
  exit 0
fi

dnd_state="$(swaync-client -D 2>/dev/null || printf 'false')"
count="$(swaync-client -c 2>/dev/null || printf '0')"

[[ $count =~ ^[0-9]+$ ]] || count=0

if [[ $dnd_state =~ ^[Tt]rue$ ]]; then
  if (( count > 0 )); then
    emit "$muted_icon $count" active "Notifications silenced\n$count notification(s) in SwayNC\nLeft-click: open panel\nMiddle-click: resume notifications\nRight-click: dismiss all"
  else
    emit "$muted_icon" active "Notifications silenced\nMiddle-click: resume notifications"
  fi
elif (( count > 0 )); then
  emit "$bell_icon $count" pending "$count notification(s) in SwayNC\nLeft-click: open panel\nMiddle-click: silence notifications\nRight-click: dismiss all"
else
  emit "" hidden "No notifications\nMiddle-click: silence notifications"
fi
