local M = {}

function M.exec_if_present(command_name, command)
  hl.exec_cmd(string.format("command -v %s >/dev/null 2>&1 && %s", command_name, command))
end

return M
