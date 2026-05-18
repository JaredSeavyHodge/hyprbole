local hyprbole_path = os.getenv("HYPRBOLE_PATH") or (os.getenv("HOME") .. "/.local/share/hyprbole")

hl.on("hyprland.start", function()
  hl.exec_cmd(hyprbole_path .. "/bin/hyprbole-waybar-restart")
  hl.exec_cmd(hyprbole_path .. "/bin/hyprbole-restart-wallpaper")
  hl.exec_cmd("blueman-applet")
end)
