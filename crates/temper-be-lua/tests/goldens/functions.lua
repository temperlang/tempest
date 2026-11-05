local temper = require("temper-core")

---@class functions.Counter
---@field apply fun(self: functions.Counter): integer

---@class functions.Counter.type: temper.Type
local Counter = temper.type("functions.Counter")

---@class functions.IntOp
---@field apply fun(self: functions.IntOp, x: integer): integer

---@class functions.IntOp.type: temper.Type
local IntOp = temper.type("functions.IntOp")

---@param start integer
---@return fun(): integer
local function makeCounter(start)
  local count = start
  ---@return integer
  local function fn()
    count = temper.int32_add(count, 1)
    return count
  end
  return fn
end

---@param f fun(arg0: integer): integer
---@return fun(arg0: integer): integer
local function twice(f)
  return function(x) return f(f(x)) end
end

---@param f fun(arg0: integer): integer
---@param x integer
---@return integer
local function applyTo(f, x)
  return f(x)
end

---@param x integer
---@return integer
local function addOne(x)
  return temper.int32_add(x, 1)
end

---@class functions.Scaler

---@class functions.Scaler.type: temper.Type
---@operator call: functions.Scaler
local Scaler = temper.type("functions.Scaler")

---@param this functions.Scaler
function Scaler.constructor(this)
end

---@param x integer
---@return integer
function Scaler.triple(x)
  return temper.int32_mul(x, 3)
end

---@class functions.Button
---@field label string
---@field onPress fun(): integer
---@field press fun(self: functions.Button): string

---@class functions.Button.type: temper.Type
---@operator call: functions.Button
local Button = temper.type("functions.Button")

---@param this functions.Button
---@param label string
---@param onPress fun(): integer
function Button.constructor(this, label, onPress)
  this.label = label
  this.onPress = onPress
end

---@param this functions.Button
---@return string
function Button.methods.press(this)
  return this.label .. " " .. this.onPress()
end

---@param steps integer
---@param body fun(): temper.Generator
local function drive(steps, body)
  local g = body()
  do
    local s = 0
    while s < steps do
      temper.generator_next(g)
      s = temper.int32_add(s, 1)
    end
  end
end

local counter = makeCounter(10)
counter()
print("count " .. counter())
local other = makeCounter(0)
print("other " .. other() .. " count " .. counter())
local addTwo = twice(addOne)
print("twice " .. addTwo(5) .. " " .. applyTo(addOne, 1))
print("triple " .. applyTo(Scaler.triple, 4) .. " " .. twice(Scaler.triple)(2))
local ns = { 5, 2, 8, 1, 4 }
local doubled = temper.listed_map(ns, function(n) return temper.int32_mul(n, 2) end)
local big = temper.listed_filter(doubled, function(n) return n > 4 end)
print(temper.listed_join(big, ", ", function(n) return tostring(n) end))
local order = temper.listed_sorted(ns, function(a, b) return temper.int32_sub(a, b) end)
print(temper.listed_join(order, " ", function(n) return tostring(n) end))
local total = 0
do
  local length = temper.list_length(ns)
  local i2 = 0
  while i2 < length do
    local n = temper.list_get(ns, i2)
    i2 = temper.int32_add(i2, 1)
    total = temper.int32_add(total, n)
  end
end
print("total " .. total)
local button = Button("hits", makeCounter(0))
button:press()
print(button:press())
local makers = temper.listbuilder_constructor()
do
  local i = 0
  while i < 3 do
    local k = temper.int32_mul(i, 10)
    temper.listbuilder_add(makers, function() return k end)
    i = temper.int32_add(i, 1)
  end
end
print(temper.listed_join(makers, " ", function(c) return tostring(c()) end))

local function fn8()
  local fs = temper.listbuilder_constructor()
  do
    local i = 0
    while i < 3 do
      local j = temper.int32_mul(i, 10)
      temper.listbuilder_add(fs, function() return j end)
      temper.yield()
      temper.listbuilder_add(fs, function() return temper.int32_add(j, 1) end)
      i = temper.int32_add(i, 1)
    end
  end
  print(temper.listed_join(fs, " ", function(c) return tostring(c()) end))
  temper.yield()
end

drive(5, temper.adapt_generator_fn(fn8))

return {}
