local temper = require("temper-core")

---@class pure-virtual.Greeter
---@field greet fun(self: pure-virtual.Greeter)

---@class pure-virtual.Greeter.type: temper.Type
local Greeter = temper.type("pure-virtual.Greeter")

---@class pure-virtual.HiGreeter: pure-virtual.Greeter
---@field greet fun(self: pure-virtual.HiGreeter)

---@class pure-virtual.HiGreeter.type: temper.Type
---@operator call: pure-virtual.HiGreeter
local HiGreeter = temper.type("pure-virtual.HiGreeter", Greeter)

---@param this pure-virtual.HiGreeter
function HiGreeter.constructor(this)
end

---@param this pure-virtual.HiGreeter
function HiGreeter.methods.greet(this)
  print("Hi, virtually!")
end

local greeter = HiGreeter()
greeter:greet()

return {}
