---@param x number
---@param y number
---@return number
local function normSquared(x, y)
  return x * x + y * y
end

return { normSquared = normSquared }
