#!/bin/bash

if command -v swaync-client >/dev/null 2>&1 && swaync-client -D | grep -qi true; then
  printf '{"text":"󰂛","class":"active","tooltip":"Notifications silenced"}\n'
else
  printf '{"text":"","class":"hidden","tooltip":"Notifications active"}\n'
fi
