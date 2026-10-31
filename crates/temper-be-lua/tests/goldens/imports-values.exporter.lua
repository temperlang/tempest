local temper = require("temper-core")

---@return integer[]
local function foo()
  return { temper.int32_add(21, 21), 42 }
end

local a = temper.list_get(foo(), 0)
local magicNumber = a

return { magicNumber = magicNumber }
