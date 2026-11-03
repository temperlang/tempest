local temper = require("temper-core")
local Checked = require("renamed").Checked
local lambda = require("renamed").lambda
local native = require("renamed").native
local sum = require("modules").sum
local twice = require("modules/helpers").twice
local base = require("modules/helpers").base

---@param n integer
---@return integer
local function both(n)
  return temper.int32_add(temper.int32_add(sum(n), twice(n)), base)
end

print("both " .. both(2))
local checked = Checked(lambda(native))
print("renamed " .. checked:delete() .. " " .. checked.local_ .. " " .. checked.end_)

return { both = both }
