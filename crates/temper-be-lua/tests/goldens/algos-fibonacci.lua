local temper = require("temper-core")

---@param i integer
---@return integer
local function fib(i)
  local a = 0
  local b = 1
  while i > 0 do
    local c = temper.int32_add(a, b)
    a = b
    b = c
    i = temper.int32_sub(i, 1)
  end
  return a
end

local zero = 0
print("fib(1)=" .. fib(temper.int32_add(zero, 1)))
print("fib(10)=" .. fib(temper.int32_add(zero, 10)))
print("fib(20)=" .. fib(temper.int32_add(zero, 20)))

return {}
