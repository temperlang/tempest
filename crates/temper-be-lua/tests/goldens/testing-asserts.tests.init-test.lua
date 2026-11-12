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

function Test_.test_module_values()
  temper.test("module values", function(test)
    do
      local length = temper.list_length(tested.unitVecs)
      local i = 0
      while i < length do
        local vec = temper.list_get(tested.unitVecs, i)
        i = temper.int32_add(i, 1)
        do
          local actual = tested.normSquared(vec)
          local expected = 1.0
          temper.test_assert(test, temper.float_eq(actual, expected), function() return "expected normSquared(vec) == (" .. temper.float64_tostring(expected) .. ") not (" .. temper.float64_tostring(actual) .. ")" end)
        end
      end
    end
  end)
end

luaunit.LuaUnit.run("--pattern", "^Test_%.", ...)
