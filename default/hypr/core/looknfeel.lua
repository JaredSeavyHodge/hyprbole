hl.config({
  general = {
    gaps_in = 6,
    gaps_out = 18,
    border_size = 2,
    layout = "dwindle",
    resize_on_border = false,
  },
  decoration = {
    rounding = 8,
    active_opacity = 1.0,
    inactive_opacity = 0.85,
    shadow = {
      enabled = true,
      range = 10,
      render_power = 3,
      color = 0xaa000000,
    },
    blur = {
      enabled = true,
      size = 5,
      passes = 2,
      xray = true,
    },
  },
  animations = {
    enabled = true,
  },
  misc = {
    force_default_wallpaper = -1,
    disable_hyprland_logo = true,
  },
  dwindle = {
    preserve_split = true,
  },
})

hl.curve("easeOutQuint", { type = "bezier", points = { {0.23, 1}, {0.32, 1} } })
hl.curve("linear", { type = "bezier", points = { {0, 0}, {1, 1} } })
hl.curve("almostLinear", { type = "bezier", points = { {0.5, 0.5}, {0.75, 1} } })
hl.curve("quick", { type = "bezier", points = { {0.15, 0}, {0.1, 1} } })
hl.curve("easy", { type = "spring", mass = 1, stiffness = 71.2633, dampening = 15.8273644 })

hl.animation({ leaf = "global", enabled = true, speed = 10, bezier = "default" })
hl.animation({ leaf = "border", enabled = true, speed = 5.39, bezier = "easeOutQuint" })
hl.animation({ leaf = "windows", enabled = true, speed = 4.79, spring = "easy" })
hl.animation({ leaf = "windowsIn", enabled = true, speed = 4.1, spring = "easy", style = "popin 87%" })
hl.animation({ leaf = "windowsOut", enabled = true, speed = 1.49, bezier = "linear", style = "popin 87%" })
hl.animation({ leaf = "fade", enabled = true, speed = 3.03, bezier = "quick" })
hl.animation({ leaf = "layers", enabled = true, speed = 3.81, bezier = "easeOutQuint" })
hl.animation({ leaf = "workspaces", enabled = true, speed = 1.94, bezier = "almostLinear", style = "fade" })
