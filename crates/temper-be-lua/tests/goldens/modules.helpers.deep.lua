---@param who string
---@return string
local function greet(who)
  return "hello, " .. who .. " from deep"
end

return { greet = greet }
