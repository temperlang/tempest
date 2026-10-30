local temper = require("temper-core")
local tested = require("testing")["$test"]
local luaunit = require("luaunit")
luaunit.FAILURE_PREFIX = temper.test_failure_prefix

Test_ = {}

function Test_.test_twice_works()
  temper.test("twice works", function(test)
    local x = tested.twice(2)
    tested.runs__set(temper.int32_add(temper.int32_add(tested.runs__get(), x), tested.base))
  end)
end

function Test_.test_names_punctuation_quotes_and_n_code()
  temper.test("names: punctuation, 'quotes', and ünïcode", function(t)
    tested.twice(tested.runs__get())
  end)
end

luaunit.LuaUnit.run("--pattern", "^Test_%.", ...)
