local temper = require("temper-core")
local sum = require("modules").sum
local twice = require("modules/helpers").twice
local base = require("modules/helpers").base

---@param n integer
---@return integer
local function both(n)
  return temper.int32_add(temper.int32_add(sum(n), twice(n)), base)
end

print("both " .. both(2))

return { both = both }
