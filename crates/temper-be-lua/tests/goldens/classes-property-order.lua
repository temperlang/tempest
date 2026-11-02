local temper = require("temper-core")

---@class property-order.C
---@field backed integer
---@field computed integer

---@class property-order.C.type: temper.Type
---@operator call: property-order.C
local C = temper.type("property-order.C")

---@param this property-order.C
---@param backed? integer|temper.Null
function C.constructor(this, backed)
  if temper.is_null(backed) then
    print("initializing backed")
    backed = 1
  end
  ---@cast backed integer
  this.backed = backed
end

---@param this property-order.C
---@return integer
function C.get.computed(this)
  print("computing computed")
  return 1
end

print("about to create instance")
local c = C()
print("about to get backed")
print("c.backed=" .. c.backed)
print("about to get computed")
print("c.computed=" .. c.computed)
print("done")

return {}
