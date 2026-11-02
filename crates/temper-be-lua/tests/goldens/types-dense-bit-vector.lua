local temper = require("temper-core")

local v = temper.densebitvector_constructor(16)
print("v[2]=" .. tostring(temper.densebitvector_get(v, 2)))
print("v[3]=" .. tostring(temper.densebitvector_get(v, 3)))
print("v[3] <- true")
temper.densebitvector_set(v, 3, true)
print("v[2]=" .. tostring(temper.densebitvector_get(v, 2)))
print("v[3]=" .. tostring(temper.densebitvector_get(v, 3)))
print("v[3] <- false")
temper.densebitvector_set(v, 3, false)
print("v[2]=" .. tostring(temper.densebitvector_get(v, 2)))
print("v[3]=" .. tostring(temper.densebitvector_get(v, 3)))

return {}
