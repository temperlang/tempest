local ExportedClass = require("types/exporter").ExportedClass

local x = ExportedClass(42)
print("foo = " .. x.foo)

return {}
