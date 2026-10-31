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
branch(0, 2)
branch(2, 0)

return {}
