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

---Null, as Temper's nullable types hold it: a value of its own, since
---Lua's `nil` can't be a table's value.
---@class temper.Null

---@type temper.Null
temper.null = {}

---Whether a value is null: `temper.null`, or `nil`.
---@param a any
---@return boolean
function temper.is_null(a) end

---Runs a function, as Lua's `pcall`: whether it ran without an error,
---as `orelse` asks of its body.
---@param f fun()
---@return boolean
function temper.pcall(f) end

---The value, if it's a string; else it bubbles.
---@param thing any
---@return string
function temper.cast_to_string(thing) end

---The value, if it's an integer; else it bubbles.
---@param thing any
---@return integer
function temper.cast_to_int(thing) end

---The value, if it's a number; else it bubbles.
---@param thing any
---@return number
function temper.cast_to_float64(thing) end

---The value, if it's a Boolean; else it bubbles.
---@param thing any
---@return boolean
function temper.cast_to_boolean(thing) end

---Core's conversions, which bubble where Temper's do.

---@param n number
---@return integer
function temper.float64_toint32(n) end

---@param n number
---@return integer
function temper.float64_toint32unsafe(n) end

---@param n number
---@return integer
function temper.float64_toint64(n) end

---@param n number
---@return integer
function temper.float64_toint64unsafe(n) end

---@param n number
---@return string
function temper.float64_tostring(n) end

---A float's sign: -1.0, 1.0, or itself for zero and NaN.
---@param x number
---@return number
function temper.float64_sign(x) end

---@param n integer
---@return number
function temper.int32_tofloat64(n) end

---@param n integer
---@return number
function temper.int64_tofloat64(n) end

---@param n integer
---@return number
function temper.int64_tofloat64unsafe(n) end

---@param n integer
---@return integer
function temper.int64_toint32unsafe(n) end

---@param n integer
---@param radix? integer
---@return string
function temper.int64_tostring(n, radix) end

---@param s string
---@return number
function temper.string_tofloat64(s) end

---@param s string
---@param radix? integer
---@return integer
function temper.string_toint32(s, radix) end

---@param s string
---@param radix? integer
---@return integer
function temper.string_toint64(s, radix) end

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

---A list's item; it bubbles out of bounds.
---@generic T
---@param list T[]
---@param index integer
---@return T
function temper.list_get(list, index) end

---A list's item; it bubbles out of bounds.
---@generic T
---@param list T[]
---@param index integer
---@return T
function temper.listed_get(list, index) end

---A list's item, or the fallback out of bounds.
---@generic T
---@param list T[]
---@param index integer
---@param orvalue T
---@return T
function temper.listed_getor(list, index, orvalue) end

---A list's length.
---@generic T
---@param list T[]
---@return integer
function temper.list_length(list) end

---A list's length.
---@generic T
---@param list T[]
---@return integer
function temper.listbuilder_length(list) end

---A list's length.
---@generic T
---@param list T[]
---@return integer
function temper.listed_length(list) end

---Whether a list is empty.
---@generic T
---@param list T[]
---@return boolean
function temper.list_isempty(list) end

---Whether a list is empty.
---@generic T
---@param list T[]
---@return boolean
function temper.listbuilder_isempty(list) end

---Whether a list is empty.
---@generic T
---@param list T[]
---@return boolean
function temper.listed_isempty(list) end

---A copy of a list's items.
---@generic T
---@param list T[]
---@return T[]
function temper.list_tolist(list) end

---A copy of a list's items.
---@generic T
---@param list T[]
---@return T[]
function temper.listbuilder_tolist(list) end

---A copy of a list's items.
---@generic T
---@param list T[]
---@return T[]
function temper.listed_tolist(list) end

---A copy of a list's items.
---@generic T
---@param list T[]
---@return T[]
function temper.list_tolistbuilder(list) end

---A copy of a list's items.
---@generic T
---@param list T[]
---@return T[]
function temper.listbuilder_tolistbuilder(list) end

---A copy of a list's items.
---@generic T
---@param list T[]
---@return T[]
function temper.listed_tolistbuilder(list) end

---A list's items from start to before stop.
---@generic T
---@param list T[]
---@param start integer
---@param stop integer
---@return T[]
function temper.listed_slice(list, start, stop) end

---A list's items, each transformed.
---@generic T, O
---@param l T[]
---@param f fun(arg0: T): O
---@return O[]
function temper.listed_map(l, f) end

---A list's items that pass a predicate.
---@generic T
---@param l T[]
---@param f fun(arg0: T): boolean
---@return T[]
function temper.listed_filter(l, f) end

---A list's items, each made a string, with a separator between.
---@generic T
---@param list T[]
---@param join string
---@param cb fun(arg0: T): string
---@return string
function temper.listed_join(list, join, cb) end

---A list's items, sorted by a comparator, stably.
---@generic T
---@param list T[]
---@param compare fun(arg0: T, arg1: T): integer
---@return T[]
function temper.listed_sorted(list, compare) end

---A list's items reduced, from its first.
---@generic T
---@param list T[]
---@param accumulate fun(arg0: T, arg1: T): T
---@return T
function temper.listed_reduce(list, accumulate) end

---A list's items reduced, from a first value.
---@generic T, O
---@param list T[]
---@param initial O
---@param accumulate fun(arg0: O, arg1: T): O
---@return O
function temper.listed_reducefrom(list, initial, accumulate) end

---Sorts a list builder in place, by a comparator, stably.
---@generic T
---@param list T[]
---@param compare fun(arg0: T, arg1: T): integer
function temper.listbuilder_sort(list, compare) end

---A new list builder.
---@return any[]
function temper.listbuilder_constructor() end

---Adds an item, at the end or at a place.
---@generic T
---@param builder T[]
---@param obj T
---@param at integer?
function temper.listbuilder_add(builder, obj, at) end

---Adds items, at the end or at a place.
---@generic T
---@param builder T[]
---@param from T[]
---@param at integer?
function temper.listbuilder_addall(builder, from, at) end

---Removes every item.
---@generic T
---@param builder T[]
function temper.listbuilder_clear(builder) end

---Removes the last item and gives it.
---@generic T
---@param builder T[]
---@return T
function temper.listbuilder_removelast(builder) end

---Reverses the items in place.
---@generic T
---@param builder T[]
function temper.listbuilder_reverse(builder) end

---Sets an item.
---@generic T
---@param builder T[]
---@param index integer
---@param value T
function temper.listbuilder_set(builder, index, value) end

---A map of pairs.
---@generic K, V
---@param list { key: K, value: V }[]
---@return table<K, V>
function temper.map_constructor(list) end

---A new map builder.
---@return table<any, any>
function temper.mapbuilder_constructor() end

---Sets a key's value.
---@generic K, V
---@param builder table<K, V>
---@param key K
---@param value V
function temper.mapbuilder_set(builder, key, value) end

---Removes a key and gives its value; it bubbles if the key is missing.
---@generic K, V
---@param builder table<K, V>
---@param key K
---@return V
function temper.mapbuilder_remove(builder, key) end

---Removes every key.
---@generic K, V
---@param builder table<K, V>
function temper.mapbuilder_clear(builder) end

---A key's value; it bubbles if the key is missing.
---@generic K, V
---@param mapped table<K, V>
---@param key K
---@return V
function temper.mapped_get(mapped, key) end

---A key's value, or the fallback.
---@generic K, V
---@param mapped table<K, V>
---@param key K
---@param otherwise V
---@return V
function temper.mapped_getor(mapped, key, otherwise) end

---Whether a map has a key.
---@generic K, V
---@param mapped table<K, V>
---@param key K
---@return boolean
function temper.mapped_has(mapped, key) end

---How many keys a map has.
---@generic K, V
---@param mapped table<K, V>
---@return integer
function temper.mapped_length(mapped) end

---A map's keys, in order.
---@generic K, V
---@param mapped table<K, V>
---@return K[]
function temper.mapped_keys(mapped) end

---A map's values, in order.
---@generic K, V
---@param mapped table<K, V>
---@return V[]
function temper.mapped_values(mapped) end

---A map's pairs, in order.
---@generic K, V
---@param mapped table<K, V>
---@return { key: K, value: V }[]
function temper.mapped_tolist(mapped) end

---A map of a map's entries.
---@generic K, V
---@param mapped table<K, V>
---@return table<K, V>
function temper.mapped_tomap(mapped) end

---A map builder of a map's entries.
---@generic K, V
---@param mapped table<K, V>
---@return table<K, V>
function temper.mapped_tomapbuilder(mapped) end

---Calls a function with each key and value.
---@generic K, V
---@param mapped table<K, V>
---@param func fun(arg0: K, arg1: V)
function temper.mapped_foreach(mapped, func) end

---A pair.
---@generic K, V
---@param a K
---@param b V
---@return { key: K, value: V }
function temper.pair_constructor(a, b) end

return temper
