local temper = require("temper-core")

---@type integer
local calls

---@param b boolean
---@return boolean
local function noisy(b)
  calls = temper.int32_add(calls, 1)
  return b
end

---@param n integer
---@return integer
local function half(n)
  if temper.int32_mod(n, 2) ~= 0 then
    return temper.bubble()
  end
  return temper.int32_div(n, 2)
end

---@param c boolean
---@param a boolean
---@param b boolean
---@return boolean
local function pick(c, a, b)
  if noisy(c) then
    return a
  else
    return b
  end
end

---@param n integer
---@return boolean
local function isPos(n)
  return n > 0
end

---@param a boolean
---@param n integer
---@return boolean
local function both(a, n)
  if a then
    ---@type integer
    local t
    if not temper.pcall(function()
      t = half(n)
    end) then
      t = 0
    end
    return isPos(t)
  else
    return false
  end
end

---@param a boolean
---@param n integer
---@return boolean
local function either(a, n)
  if a then
    return true
  else
    ---@type integer
    local t
    if not temper.pcall(function()
      t = half(n)
    end) then
      t = 0
    end
    return isPos(t)
  end
end

---@param a integer
---@param b integer
---@return integer
local function sum(a, b)
  return temper.int32_add(a, b)
end

---@param x string|temper.Null
---@return string
local function label(x)
  ---@type integer
  local t
  if not temper.pcall(function()
    t = half(3)
  end) then
    t = -1
  end
  return "got " .. (temper.is_null(x) and "nothing" or x --[[@as string]]) .. " and " .. t
end

calls = 0
print(tostring(pick(true, false, true)) .. " " .. tostring(pick(false, false, true)) .. " " .. calls)
print(tostring(both(true, 4)) .. " " .. tostring(both(true, 3)) .. " " .. tostring(both(false, 4)))
print(tostring(either(false, 4)) .. " " .. tostring(either(false, 3)) .. " " .. tostring(either(true, 3)))
---@type integer
local t
if not temper.pcall(function()
  t = half(8)
end) then
  t = 0
end
---@type integer
local t_1
if not temper.pcall(function()
  t_1 = half(7)
end) then
  t_1 = 100
end
print(tostring(sum(t, t_1)))
print(label(temper.null))
print(label("x"))

return {}
