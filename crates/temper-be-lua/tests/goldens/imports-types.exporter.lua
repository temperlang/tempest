local temper = require("temper-core")

---@class types.exporter.ExportedClass
---@field foo integer

---@class types.exporter.ExportedClass.type: temper.Type
---@operator call: types.exporter.ExportedClass
local ExportedClass = temper.type("types.exporter.ExportedClass")

---@param this types.exporter.ExportedClass
---@param foo integer
function ExportedClass.constructor(this, foo)
  this.foo = foo
end

return { ExportedClass = ExportedClass }
