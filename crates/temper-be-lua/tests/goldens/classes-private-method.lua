local temper = require("temper-core")

---@class private-method.C
---@field helper fun(self: private-method.C, x: integer): integer
---@field method1 fun(self: private-method.C, x: integer): integer
---@field method2 fun(self: private-method.C, x: integer): integer

---@class private-method.C.type: temper.Type
---@operator call: private-method.C
local C = temper.type("private-method.C")

---@param this private-method.C
function C.constructor(this)
end

---@param this private-method.C
---@param x integer
---@return integer
function C.methods.helper(this, x)
  return temper.int32_add(x, 1)
end

---@param this private-method.C
---@param x integer
---@return integer
function C.methods.method1(this, x)
  return this:helper(temper.int32_mul(x, 2))
end

---@param this private-method.C
---@param x integer
---@return integer
function C.methods.method2(this, x)
  return this:helper(temper.int32_mul(x, 3))
end

local c = C()
print("method1(10)=" .. c:method1(10))
print("method2(10)=" .. c:method2(10))

return {}
