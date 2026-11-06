local temper = require("temper-core")

local parseGrid

---Gives a class's instances a `__tostring` that calls their `toString`.
---@param class temper.Type
local function with_tostring(class)
  local construct = class.constructor
  ---@type table?
  local meta = nil
  class.constructor = function(this, ...)
    if meta == nil then
      meta = {}
      for k, v in _G.pairs(_G.getmetatable(this)) do
        meta[k] = v
      end
      meta.__tostring = function(self)
        return self:toString()
      end
    end
    _G.setmetatable(this, meta)
    construct(this, ...)
  end
end

---@class libraries.Grid
---@field ncols integer
---@field values number[]
---@field get fun(self: libraries.Grid, i: integer, j: integer): number

---@class libraries.Grid.type: temper.Type
---@operator call: libraries.Grid
local Grid = temper.type("libraries.Grid")

---@param this libraries.Grid
---@param ncols integer
---@param values number[]
function Grid.constructor(this, ncols, values)
  this.ncols = ncols
  this.values = values
end

---@param this libraries.Grid
---@param i integer
---@param j integer
---@return number
function Grid.methods.get(this, i, j)
  return temper.list_get(this.values, temper.int32_add(temper.int32_mul(i, this.ncols), j))
end

---@param s string
---@return libraries.Grid|temper.Null
function Grid.parse(s)
  ---@type libraries.Grid|temper.Null
  local t
  if not temper.pcall(function()
    t = parseGrid(s)
  end) then
    t = temper.null
  end
  return t
end

---@param s string
---@return libraries.Grid
function parseGrid(s)
  if s == "" then
    return temper.bubble()
  end
  return Grid(1, { temper.string_tofloat64(s) })
end

---@class libraries.Casing
---@field name string
---@field eq fun(self: libraries.Casing, other: libraries.Casing): boolean
---@field toString fun(self: libraries.Casing): string

---@class libraries.Casing.type: temper.Type
---@field first libraries.Casing
---@operator call: libraries.Casing
local Casing = temper.type("libraries.Casing")

---@param this libraries.Casing
---@param other libraries.Casing
---@return boolean
function Casing.methods.eq(this, other)
  return this.name == other.name
end

---@param this libraries.Casing
---@param name string
function Casing.constructor(this, name)
  this.name = name
end

---@param this libraries.Casing
---@return string
function Casing.methods.toString(this)
  return this.name
end

with_tostring(Casing)

---@param values number[]
---@param ncols integer
---@return integer
local function rows(values, ncols)
  ---@type integer
  local t
  if not temper.pcall(function()
    t = temper.int32_div(temper.list_length(values), ncols)
  end) then
    error("panic")
  end
  return t
end

---@param grid libraries.Grid
---@return libraries.Grid
local function scaled(grid)
  if grid.ncols == 0 then
    return temper.bubble()
  end
  return grid
end

---@param grid libraries.Grid
---@return number[]
local function cubes(grid)
  ---@type number[]
  local t
  if not temper.pcall(function()
    local more = temper.listed_map(scaled(grid).values, function(x) return x * x * x end)
    t = more
  end) then
    error("panic")
  end
  return t
end

---@param values number[]
---@param f fun(arg0: number)
local function each(values, f)
  do
    local i = 0
    while i < temper.list_length(values) do
      f(temper.list_get(values, i))
      i = temper.int32_add(i, 1)
    end
  end
end

---@param values number[]
---@return number
local function total(values)
  local sum = 0.0
  ---@param x number
  local function fn(x)
    if not temper.pcall(function()
      local part = x / 2.0
      sum = sum + part
    end) then
      error("panic")
    end
  end
  each(values, fn)
  return sum
end

---@param x number
---@return number
local function half(x)
  ---@type number
  local t
  if not temper.pcall(function()
    t = x / 2.0
  end) then
    error("panic")
  end
  return t
end

---@param x number
---@return number
local function cube(x)
  return x * 0.25
end

---@param grid libraries.Grid
---@return number
local function corner(grid)
  ---@type number
  local t
  if not temper.pcall(function()
    t = grid:get(1, 1)
  end) then
    t = -1.0
  end
  return t
end

---@param n integer
---@return libraries.Grid|temper.Null
local function maybe(n)
  if n > 0 then
    return Grid(n, { 1.0 })
  else
    return temper.null
  end
end

---@param grid libraries.Grid
---@return number[]
local function mapped(grid)
  ---@type libraries.Grid
  local t
  if not temper.pcall(function()
    t = scaled(grid)
  end) then
    error("panic")
  end
  ---@param x number
  ---@return number
  local function fn(x)
    if temper.float_cmp(x, 1.5) > 0 then
      return x * 2.0
    else
      return x
    end
  end
  return temper.listed_map(t.values, fn)
end

---@param n integer
---@return string|temper.Null
local function text(n)
  if n > 0 then
    return "x"
  else
    return temper.null
  end
end

---@param n integer
---@return integer|temper.Null
local function count(n)
  if n > 0 then
    return n
  else
    return temper.null
  end
end

---@param n integer
---@return libraries.Casing|temper.Null
local function casing(n)
  if n > 0 then
    return Casing.first
  else
    return temper.null
  end
end

---@param s string
---@return string
local function captured(s)
  ---@type libraries.Grid|temper.Null
  local found
  if not temper.pcall(function()
    found = parseGrid(s)
  end) then
    found = temper.null
  end
  return temper.listed_join(temper.listed_map({ 1, 2 }, function(x) return temper.is_null(found) end), ",", function(b) return tostring(b) end)
end

---@return string
local function bidi()
  return "look\226\128\174out"
end

---@param n integer
---@return string
local function word(n)
  if n > 1 then
    return "b"
  else
    return "a"
  end
end

Casing.first = Casing("first")
local grid = Grid(2, { 1.0, 2.0, 3.0, 4.0 })
print(rows(grid.values, 2) .. " " .. temper.float64_tostring(half(3.0)) .. " " .. temper.float64_tostring(cube(8.0)))
print(temper.float64_tostring(corner(grid)))
print(temper.float64_tostring(total(grid.values)))
local none = text(0)
local some = count(3)
local part = none
local part2 = some
local subject = maybe(0)
local part3 = temper.is_null(subject) and temper.null or (subject --[[@as libraries.Grid]]).ncols
print((temper.is_null(part) and "null" or part --[[@as string]]) .. " " .. (temper.is_null(part2) and "null" or tostring(part2 --[[@as integer]])) .. " " .. (temper.is_null(part3) and "null" or tostring(part3 --[[@as integer]])))
print(tostring(not temper.is_null(maybe(1))) .. " " .. tostring(not temper.is_null(maybe(0))) .. " " .. tostring(temper.is_null(Grid.parse(""))))
print(Casing.first:toString())
print(temper.listed_join(mapped(grid), ",", function(x) return temper.float64_tostring(x) end))
print(temper.listed_join(cubes(grid), ",", function(x) return temper.float64_tostring(x) end))
local left = casing(1)
local left2 = casing(0)
local left3 = casing(0)
local right = casing(0)
print(tostring(not temper.is_null(left) and (left --[[@as libraries.Casing]]):eq(Casing.first)) .. " " .. tostring(not temper.is_null(left2) and (left2 --[[@as libraries.Casing]]):eq(Casing.first)) .. " " .. tostring(not (temper.is_null(left3) and temper.is_null(right) or not temper.is_null(left3) and (not temper.is_null(right) and (left3 --[[@as libraries.Casing]]):eq(right --[[@as libraries.Casing]])))))
print(captured("") .. " " .. captured("2.5"))
print(tostring(bidi() ~= "lookout"))
print(temper.str_cmp(word(1), word(2)) .. " " .. temper.str_cmp(word(2), word(1)) .. " " .. temper.str_cmp(word(2), word(3)))
local named = word(2)
print(tostring(named == word(3) and named .. "!" ~= named))

return { Grid = Grid, parseGrid = parseGrid, Casing = Casing }
