local temper = require("temper-core")

---@param b integer[]
local function f(b)
  if temper.listed_isempty(b) then
    print("empty")
  else
    print("not empty")
  end
end

f({ 2, 3 })
f({})

return {}
