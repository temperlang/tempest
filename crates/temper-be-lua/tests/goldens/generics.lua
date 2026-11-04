local temper = require("temper-core")

---@class generics.Holder
---@field held any

---@class generics.Holder.type: temper.Type
local Holder = temper.type("generics.Holder")

---@class generics.Box: generics.Holder
---@field value any
---@field held any
---@field pairWith fun(self: generics.Box, other: integer): { key: any, value: integer }

---@class generics.Box.type: temper.Type
---@operator call: generics.Box
local Box = temper.type("generics.Box", Holder)

---@param this generics.Box
---@param value any
function Box.constructor(this, value)
  this.value = value
end

---@param this generics.Box
---@return any
function Box.get.held(this)
  return this.value
end

---@param this generics.Box
---@param other integer
---@return { key: any, value: integer }
function Box.methods.pairWith(this, other)
  return temper.pair_constructor(this.value, other)
end

---@class generics.Named
---@field name string

---@class generics.Named.type: temper.Type
---@operator call: generics.Named
local Named = temper.type("generics.Named")

---@param this generics.Named
---@param name string
function Named.constructor(this, name)
  this.name = name
end

---@class generics.Shelf
---@field items any[]
---@field firstName fun(self: generics.Shelf): string|temper.Null

---@class generics.Shelf.type: temper.Type
---@operator call: generics.Shelf
local Shelf = temper.type("generics.Shelf")

---@param this generics.Shelf
---@param items any[]
function Shelf.constructor(this, items)
  this.items = items
end

---@param this generics.Shelf
---@return string|temper.Null
function Shelf.methods.firstName(this)
  if temper.listed_isempty(this.items) then
    return temper.null
  else
    return temper.list_get(this.items, 0).name
  end
end

---@generic T
---@param items T[]
---@param fallback T
---@return T
local function first(items, fallback)
  if temper.listed_isempty(items) then
    return fallback
  else
    return temper.listed_get(items, 0)
  end
end

---@generic T
---@param h generics.Holder
---@return T
local function held(h)
  return h.held
end

---@param ns integer[]
---@return integer
local function total(ns)
  local sum = 0
  do
    local i = 0
    while i < temper.list_length(ns) do
      sum = temper.int32_add(sum, temper.list_get(ns, i))
      i = temper.int32_add(i, 1)
    end
  end
  return sum
end

---@param a integer[]
---@param b integer[]
---@return boolean
local function firstSame(a, b)
  return temper.list_get(a, 0) == temper.list_get(b, 0)
end

local ns = { 1, 2, 3 }
local none = {}
print(total(ns) .. " " .. first(ns, 0) .. " " .. first(none, -1))
print(first({ "a", "b" }, "z"))
local box = Box("boxed")
print(held(box))
local pair = box:pairWith(7)
print(pair.key .. " " .. pair.value)
local shelf = Shelf({ Named("ann"), Named("bo") })
local subject = shelf:firstName()
print(temper.is_null(subject) and "none" or subject --[[@as string]])
local noNames = {}
local empty = Shelf(noNames)
local subject2 = empty:firstName()
print(temper.is_null(subject2) and "none" or subject2 --[[@as string]])
local builder = temper.listbuilder_constructor()
temper.listbuilder_add(builder, "x")
temper.listbuilder_add(builder, "y")
temper.listbuilder_add(builder, "w", 0)
print(temper.listbuilder_length(builder) .. " " .. temper.listed_get(builder, 0) .. " " .. temper.listbuilder_removelast(builder))
local built = temper.listbuilder_tolist(builder)
print(temper.list_length(built) .. " " .. temper.listed_getor(built, 5, "?") .. " " .. temper.list_get(temper.listed_slice(built, 1, 2), 0))
local m = temper.map_constructor({ temper.pair_constructor("a", 1), temper.pair_constructor("b", 2) })
---@type integer
local t
if not temper.pcall(function()
  t = temper.mapped_get(m, "b")
end) then
  t = -1
end
---@type integer
local t_1
if not temper.pcall(function()
  t_1 = temper.mapped_get(m, "c")
end) then
  t_1 = -1
end
print(t .. " " .. t_1 .. " " .. tostring(temper.mapped_has(m, "a")) .. " " .. temper.mapped_length(m))
local mb = temper.mapbuilder_constructor()
temper.mapbuilder_set(mb, "q", 5)
temper.mapbuilder_set(mb, "n", temper.null)
local subject3 = temper.mapped_getor(mb, "q", 0)
local subject4 = temper.mapped_getor(mb, "n", 0)
print((temper.is_null(subject3) and -2 or subject3 --[[@as integer]]) .. " " .. (temper.is_null(subject4) and -2 or subject4 --[[@as integer]]) .. " " .. tostring(temper.mapped_has(mb, "n")))
---@type integer|temper.Null
local subject6
if not temper.pcall(function()
  subject6 = temper.mapbuilder_remove(mb, "q")
end) then
  subject6 = 0
end
local subject5 = temper.is_null(subject6) and temper.null or tostring(subject6 --[[@as integer]])
print(temper.is_null(subject5) and "null" or subject5 --[[@as string]])
print(tostring(temper.listed_length(temper.mapped_keys(mb))))
print(tostring(firstSame({ 1000 }, { 1000 })) .. " " .. tostring(firstSame({ 1000 }, { 7 })))

return {
  Holder = Holder,
  Box = Box,
  Named = Named,
  Shelf = Shelf,
  first = first,
}
