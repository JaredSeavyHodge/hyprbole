#!/bin/bash

if systemctl --user is-active hypridle.service >/dev/null 2>&1; then
  printf '{"text":"󰈈","class":"active","tooltip":"Idle enabled"}\n'
else
  printf '{"text":"","class":"hidden","tooltip":"Idle disabled"}\n'
fi
