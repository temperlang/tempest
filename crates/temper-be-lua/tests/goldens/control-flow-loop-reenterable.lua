local temper = require("temper-core")

local i = 0
while i < 2 do
  local x = i
  print(tostring(x))
  i = temper.int32_add(i, 1)
end

return {}
