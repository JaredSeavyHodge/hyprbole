-- Hyprbole Hyprland entrypoint.

package.path = os.getenv("HOME")
  .. "/.config/?.lua;"
  .. (os.getenv("HYPRBOLE_PATH") or (os.getenv("HOME") .. "/.local/share/hyprbole"))
  .. "/?.lua;"
  .. package.path

require("default.hypr.hyprbole")

require("hypr.monitors")
require("hypr.input")
require("hypr.bindings")
require("hypr.looknfeel")
require("hypr.autostart")

pcall(require, "hypr.overrides")
