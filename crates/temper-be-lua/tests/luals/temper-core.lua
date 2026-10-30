---@meta temper-core

-- The parts of `temper-core`, Kotlin Temper's Lua runtime, that generated
-- code calls, with their types, for LuaLS's checker (the runtime has no
-- annotations, and copies its integer helpers into its table in a loop,
-- which LuaLS can't follow). The tests check generated code against this
-- in place of the runtime, and check that the runtime defines everything
-- here (tests/tables.rs).

---@class temper-core
local temper = {}

---@param a integer
---@param b integer
---@return integer
function temper.int32_add(a, b) end

---@param a integer
---@param b integer
---@return integer
function temper.int32_sub(a, b) end

---@param a integer
---@param b integer
---@return integer
function temper.int32_mul(a, b) end

---@param a integer
---@param b integer
---@return integer
function temper.int32_div(a, b) end

---@param a integer
---@param b integer
---@return integer
function temper.int32_mod(a, b) end

---@param a integer
---@return integer
function temper.int32_unm(a) end

---@param a integer
---@param b integer
---@return integer
function temper.int_cmp(a, b) end

---@param a integer
---@param b integer
---@return integer
function temper.int64_div(a, b) end

---@param a integer
---@param b integer
---@return integer
function temper.int64_mod(a, b) end

---@param a integer
---@return integer
function temper.int64_unm(a) end

---@param a integer
---@param b integer
---@return integer
function temper.int64_cmp(a, b) end

---@param a integer
---@param b integer
---@return integer
function temper.band(a, b) end

---@param a integer
---@param b integer
---@return integer
function temper.bor(a, b) end

---@param a integer
---@param b integer
---@return integer
function temper.bxor(a, b) end

---@param a integer
---@return integer
function temper.bnot(a) end

---@param a integer
---@param b integer
---@return integer
function temper.shl32(a, b) end

---@param a integer
---@param b integer
---@return integer
function temper.shr32(a, b) end

---@param a integer
---@param b integer
---@return integer
function temper.ushr32(a, b) end

---@param a integer
---@param b integer
---@return integer
function temper.shl64(a, b) end

---@param a integer
---@param b integer
---@return integer
function temper.shr64(a, b) end

---@param a integer
---@param b integer
---@return integer
function temper.ushr64(a, b) end

---@param a number
---@param b number
---@return number
function temper.fmod(a, b) end

---@param a number
---@param b number
---@return boolean
function temper.float_eq(a, b) end

---@param a number
---@param b number
---@return integer
function temper.float_cmp(a, b) end

---@param a string
---@param b string
---@return integer
function temper.str_cmp(a, b) end

---@param a boolean
---@param b boolean
---@return integer
function temper.bool_cmp(a, b) end

---@param n integer
---@param radix integer
---@return string
function temper.int32_tostring(n, radix) end

---Raises Temper's bubble, as an error; it never returns.
---@return any
function temper.bubble() end

---What a test failure's message starts with, for LuaUnit.
---@type string
temper.test_failure_prefix = ""

---Runs a test's body with a new test, failing if it errs or the test has
---failures.
---@param name string
---@param body fun(test: table)
function temper.test(name, body) end

---A class's or interface's table, as `temper.type` makes it: its getters,
---setters, and methods, by name, its constructor, and its statics.
---@class temper.Type
---@field get table<string, fun(this: any): any>
---@field set table<string, fun(this: any, value: any)>
---@field methods table<string, function>
---@field constructor fun(this: any, ...: any)
---@field typename string
---@field [string] any

---Makes a type, with its supertypes' getters, setters, and methods.
---@param typename string
---@param ... temper.Type
---@return temper.Type
function temper.type(typename, ...) end

---Whether a value is an instance of any of the types.
---@param thing any
---@param ... temper.Type
---@return boolean
function temper.instance_of(thing, ...) end

---The value, if it's an instance of the type; else it bubbles.
---@param thing any
---@param type temper.Type
---@return any
function temper.cast_to(thing, type) end

return temper
