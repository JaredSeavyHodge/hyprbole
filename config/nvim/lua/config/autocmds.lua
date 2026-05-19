local yank_group = vim.api.nvim_create_augroup("hyprbole-yank-highlight", { clear = true })

vim.api.nvim_create_autocmd("TextYankPost", {
  group = yank_group,
  callback = function()
    vim.highlight.on_yank({ timeout = 180 })
  end,
})
