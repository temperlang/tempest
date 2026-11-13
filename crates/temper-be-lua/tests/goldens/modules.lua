local temper = require("temper-core")
require("modules/state")
require("modules/parts/regex")
local greet = require("modules/helpers/deep").greet
local twice = require("modules/helpers").twice
local base = require("modules/helpers").base
local bump = require("modules/helpers").bump
local share = require("modules/core").share
local describe = require("modules/machine").describe

---@type integer
local base2

---@class modules.Engine
---@field name string

---@class modules.Engine.type: temper.Type
---@operator call: modules.Engine
local Engine = temper.type("modules.Engine")

---@param this modules.Engine
---@param name string
function Engine.constructor(this, name)
  this.name = name
end

---@param n integer
---@return integer
local function sum(n)
  return temper.int32_add(temper.int32_add(twice(n), base), base2)
end

local calls2 = require("modules/helpers").calls
base2 = 1
print(greet("modules"))
print("sum " .. sum(3))
print("share " .. share(7, 2))
print(Engine("own").name .. ", " .. describe())
bump()
bump()
print("calls " .. calls2)
print("calls now " .. bump())

return { sum = sum }
