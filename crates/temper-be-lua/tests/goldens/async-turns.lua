local temper = require("temper-core")

---@type async-turns.Counter
local c
---@type temper.PromiseBuilder
local gateA
---@type temper.PromiseBuilder
local gateB

---@param n integer
---@return temper.Promise
local function later(n)
  local pb = temper.promisebuilder_constructor()
  local function fn()
    temper.promisebuilder_complete(pb, temper.int32_mul(n, 2))
  end
  temper.async(temper.adapt_generator_fn(fn))
  return temper.promisebuilder_promise(pb)
end

---@class async-turns.Tick
---@field apply fun(self: async-turns.Tick, i: integer)

---@class async-turns.Tick.type: temper.Type
local Tick = temper.type("async-turns.Tick")

---@param body fun(): temper.Generator
---@return temper.Generator
local function generated(body)
  return body()
end

---@param n integer
---@param each fun(arg0: integer)
---@return temper.Generator
local function ticks(n, each)
  local function fn()
    do
      local i = 0
      while i < n do
        each(i)
        temper.yield()
        i = temper.int32_add(i, 1)
      end
    end
  end
  return generated(temper.adapt_generator_fn(fn))
end

---@class async-turns.Counter
---@field n integer
---@field bump fun(self: async-turns.Counter, tag: string, times: integer)

---@class async-turns.Counter.type: temper.Type
---@operator call: async-turns.Counter
local Counter = temper.type("async-turns.Counter")

---@param this async-turns.Counter
function Counter.constructor(this)
  this.n = 0
end

---@param this async-turns.Counter
---@param tag string
---@param times integer
function Counter.methods.bump(this, tag, times)
  print(tag .. " start")
  do
    local i = 0
    while i < times do
      local before = this.n
      this.n = temper.int32_add(before, 1)
      i = temper.int32_add(i, 1)
    end
  end
  print(tag .. " done n=" .. this.n)
end

local t = ticks(2, function(i) print("tick " .. i) end)
temper.generator_next(t)
temper.generator_next(t)
temper.generator_next(t)
c = Counter()
gateA = temper.promisebuilder_constructor()
gateB = temper.promisebuilder_constructor()

local function fn2()
  c:bump("A", 10000)
  ---@type integer
  local k
  if not temper.pcall(function()
    k = temper.promisebuilder_promise(gateA):await()
  end) then
    k = -1
  end
  c:bump("A" .. k, 10000)
end

temper.async(temper.adapt_generator_fn(fn2))

local function fn3()
  c:bump("B", 10000)
  ---@type integer
  local k
  if not temper.pcall(function()
    k = temper.promisebuilder_promise(gateB):await()
  end) then
    k = -1
  end
  c:bump("B" .. k, 10000)
end

temper.async(temper.adapt_generator_fn(fn3))

local function fn4()
  print("opener: calling complete")
  temper.promisebuilder_complete(gateA, 1)
  temper.promisebuilder_complete(gateB, 2)
  print("opener: complete returned")
end

temper.async(temper.adapt_generator_fn(fn4))

return { later = later, Tick = Tick, ticks = ticks }
