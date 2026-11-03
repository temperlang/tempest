local temper = require("temper-core")
local tested = require("asserts")["$test"]
local luaunit = require("luaunit")
luaunit.FAILURE_PREFIX = temper.test_failure_prefix

Test_ = {}

function Test_.test_a_test_case()
  temper.test("a test case", function(test)
    temper.test_assert(test, temper.float_eq(tested.normSquared(tested.Vec2(0.0, 0.0)), 0.0), function() return "Oh noes" end)
  end)
end

function Test_.test_a_failing_test()
  temper.test("a failing test", function(test)
    temper.test_assert(test, false, function() return "because it's wrong" end)
    local helper = tested.TestHelper(3, 2)
    tested.assertMoreThings(test, helper, { { "bye" } })
  end)
end

function Test_.test_exported()
  temper.test("exported", function(test)
    local vec = tested.Vec2(1.5, 2.5)
    do
      local actual = vec:toString()
      local expected = "(1.5, 2.5) size squared 8.5"
      temper.test_assert(test, actual == expected, function() return "expected vec.toString() == (" .. expected .. ") not (" .. actual .. ")" end)
    end
  end)
end

luaunit.LuaUnit.run("--pattern", "^Test_%.", ...)
