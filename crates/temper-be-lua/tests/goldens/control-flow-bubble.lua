local temper = require("temper-core")

---@param x integer|temper.Null
---@return integer
local function more(x)
  return temper.bubble()
end

---@param x string|temper.Null
---@return string
local function something(x)
  if not temper.is_null(x) then
    return x --[[@as string]]
  else
    return temper.bubble()
  end
end

---@param x string|temper.Null
local function report(x)
  ---@type string
  local message
  if not temper.pcall(function()
    message = something(x)
  end) then
    message = "null"
  end
  ---@type string
  local suffix
  if not temper.pcall(function()
    ---@type integer
    local t
    if not temper.pcall(function()
      temper.cast_to_string(x)
      t = 1
    end) then
      t = 0
    end
    suffix = tostring(more(t))
  end) then
    suffix = ""
  end
  print(message .. suffix)
end

---@param cols integer
---@param total integer
---@return integer
local function calcRows(cols, total)
  print("calcRows(" .. cols .. ", " .. total .. ")")
  if temper.int32_mod(total, cols) ~= 0 then
    return temper.bubble()
  end
  return temper.int32_div(total, cols)
end

---@param s string
---@return integer
local function countBelow(s)
  local i = 0
  while i < temper.string_toint32(s) do
    i = temper.int32_add(i, 1)
  end
  return i
end

---@param n integer
---@return integer
local function firstOver(n)
  local i = 0
  while i < 9 do
    i = temper.int32_add(i, 1)
    if i > n then
      break
    end
  end
  return i
end

---@param n integer
---@return integer
local function sumSkipping(n)
  local sum = 0
  do
    local i = 0
    while i < 5 do
      if i == n then
        i = temper.int32_add(i, 1)
        goto continue
      end
      sum = temper.int32_add(sum, i)
      i = temper.int32_add(i, 1)
      ::continue::
    end
  end
  return sum
end

---@param n integer
---@return integer
local function tenTimes(n)
  do
    local i = 0
    while i < 9 do
      if i == n then
        return temper.int32_mul(i, 10)
      end
      i = temper.int32_add(i, 1)
    end
  end
  return temper.bubble()
end

---@param r integer|temper.Null
---@return boolean
local function isNone(r)
  return temper.is_null(r)
end

---@param n integer
---@return string
local function viaOrElse(n)
  ---@type integer
  local t
  if not temper.pcall(function()
    t = firstOver(n)
  end) then
    t = -1
  end
  ---@type integer
  local t_1
  if not temper.pcall(function()
    t_1 = sumSkipping(n)
  end) then
    t_1 = -1
  end
  ---@type integer
  local t_2
  if not temper.pcall(function()
    t_2 = tenTimes(n)
  end) then
    t_2 = -1
  end
  return t .. " " .. t_1 .. " " .. t_2
end

---@param a integer
---@param b integer
local function branch(a, b)
  local c = a == 0 and (b == 0 and "00" or (b == 1 and "01" or (b == 2 and "02" or "0x"))) or (a == 1 and "1x" or (a == 2 and "2x" or (a == 3 and "3x" or "xx")))
  print("branch(" .. a .. ", " .. b .. ") -> " .. c)
end

report("hi")
report(temper.null)
print(tostring(calcRows(3, 6)))
---@type integer
local t
if not temper.pcall(function()
  t = countBelow("abc")
end) then
  t = -1
end
print(tostring(t))
---@type integer
local t_1
if not temper.pcall(function()
  t_1 = firstOver(1)
end) then
  t_1 = -1
end
---@type integer|temper.Null
local t_2
if not temper.pcall(function()
  t_2 = firstOver(1)
end) then
  t_2 = temper.null
end
print(t_1 .. " " .. tostring(isNone(t_2)))
print(viaOrElse(2))
print(viaOrElse(12))
branch(0, 2)
branch(2, 0)

return {}
