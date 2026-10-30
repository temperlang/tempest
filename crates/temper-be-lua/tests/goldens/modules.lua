local temper = require("temper-core")
local greet = require("modules/helpers/deep").greet
local twice = require("modules/helpers").twice
local base = require("modules/helpers").base
local bump = require("modules/helpers").bump

---@type integer
local base2

---@param n integer
---@return integer
local function sum(n)
  return temper.int32_add(temper.int32_add(twice(n), base), base2)
end

local calls2 = require("modules/helpers").calls
base2 = 1
print(greet("modules"))
print("sum " .. sum(3))
bump()
bump()
print("calls " .. calls2)
print("calls now " .. bump())

return { sum = sum }
