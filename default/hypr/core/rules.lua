hl.window_rule({
  name = "suppress-maximize-events",
  match = { class = ".*" },
  suppress_event = "maximize",
})

hl.window_rule({
  name = "fix-xwayland-drags",
  match = {
    class = "^$",
    title = "^$",
    xwayland = true,
    float = true,
    fullscreen = false,
    pin = false,
  },
  no_focus = true,
})

hl.window_rule({
  name = "float-hyprbole-terminals",
  match = { class = "^com\\.hyprbole\\..*" },
  float = true,
})

hl.window_rule({
  name = "float-update-terminal",
  match = { class = "^com\\.hyprbole\\.update$" },
  float = true,
})

hl.window_rule({
  name = "float-satty",
  match = { class = "^com\\.gabm\\.satty$" },
  float = true,
})

hl.window_rule({
  name = "protect-1password-screen-share",
  match = { class = "^(1[pP]assword)$" },
  no_screen_share = true,
})

hl.window_rule({
  name = "send-1password-to-scratchpad",
  match = { class = "^(1[pP]assword)$" },
  workspace = "special:scratchpad",
})

hl.window_rule({
  name = "float-1password",
  match = { class = "^(1[pP]assword)$" },
  float = true,
})
