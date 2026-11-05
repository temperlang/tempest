local temper = require("temper-core")

---@param body fun(): temper.Generator
local function f(body)
  local generator = body()
  print("f0")
  temper.generator_next(generator)
  print("f1")
  temper.generator_next(generator)
  print("f2")
  temper.generator_next(generator)
  print("f3")
end

local function fn()
  local i = 0
  while true do
    print("generated " .. i)
    i = temper.int32_add(i, 1)
    temper.yield()
  end
end

f(temper.adapt_generator_fn(fn))

return {}
