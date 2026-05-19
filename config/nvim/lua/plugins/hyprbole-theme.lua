local theme_path = vim.fn.expand("~/.config/hyprbole/current/theme/neovim.lua")

if (vim.uv or vim.loop).fs_stat(theme_path) then
  local ok, spec = pcall(dofile, theme_path)
  if ok and type(spec) == "table" then
    return spec
  end

  vim.notify("Failed to load Hyprbole Neovim theme: " .. theme_path, vim.log.levels.WARN)
end

return {}
