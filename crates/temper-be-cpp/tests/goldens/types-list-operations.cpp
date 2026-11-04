// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace operations {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

}  // namespace operations

// init.cpp
#include <operations/init.hpp>

namespace operations {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;

}  // namespace temper_globals

template <class T>
static void reportLength(std::shared_ptr<std::vector<T>> const & list) {
  core::Console::log(temper_globals::console.value, "Length: " + std::to_string(core::List::length<T>(list)));
}

static void reportSplit(std::string string, std::string sep) {
  std::shared_ptr<std::vector<std::string>> strings = core::String::split(string, sep);
  std::string len = std::to_string(core::List::length<std::string>(strings));
  core::Console::log(temper_globals::console.value, "Split " + core::List::join(strings, ", ", [](std::string x) -> std::string { return x; }) + " - " + len);
}

static void something(std::function<void(std::shared_ptr<std::vector<std::string>>)> const & callback) {
  callback(core::List::make<std::string>("Called", "with", "listed."));
}

static void runSomething() {
  operations::something([](std::shared_ptr<std::vector<std::string>> words) -> void { core::Console::log(temper_globals::console.value, core::List::join(words, " ", [](std::string word) -> std::string { return word; })); });
}

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::console.value = core::Console::get_console();
    operations::reportLength<std::shared_ptr<core::AnyValueBase>>(core::List::make<std::shared_ptr<core::AnyValueBase>>(core::any_box(4), core::any_box(5), core::any_box(6)));
    std::shared_ptr<std::vector<int32_t>> ls = core::List::sorted(core::List::make<int32_t>(5, 6, 7, 8, 9, 0, 1, 2, 3, 4), [](int32_t i, int32_t j) -> int32_t { return core::Int::sub(i, j); });
    core::Console::log(temper_globals::console.value, "All  " + core::List::join(ls, ", ", [](int32_t x) -> std::string { return std::to_string(x); }));
    core::Console::log(temper_globals::console.value, "Even " + core::List::join(core::List::filter(ls, [](int32_t x) -> bool { return (x & 1) == 0; }), ", ", [](int32_t x) -> std::string { return std::to_string(x); }));
    core::Console::log(temper_globals::console.value, "Odd  " + core::List::join(core::List::filter(ls, [](int32_t x) -> bool { return (x & 1) != 0; }), ", ", [](int32_t x) -> std::string { return std::to_string(x); }));
    core::Console::log(temper_globals::console.value, "Neg  " + core::List::join(core::List::map(ls, [](int32_t x) -> int32_t { return core::Int::neg(x); }), ", ", [](int32_t x) -> std::string { return std::to_string(x); }));
    std::shared_ptr<std::vector<int32_t>> listed = ls;
    core::Console::log(temper_globals::console.value, "Neg  " + core::List::join(core::List::map(listed, [](int32_t x) -> int32_t { return core::Int::neg(x); }), ", ", [](int32_t x) -> std::string { return std::to_string(x); }));
    std::shared_ptr<std::vector<int32_t>> listedList = core::List::toList<int32_t>(listed);
    core::Console::log(temper_globals::console.value, "Listed list length is " + std::to_string(core::List::length<int32_t>(listedList)));
    core::Console::log(temper_globals::console.value, "Looping");
    int32_t n = core::List::length<int32_t>(ls);
    {
      int32_t i = 0;
      while (i < n) {
        core::Console::log(temper_globals::console.value, "  " + std::to_string(core::List::get<int32_t>(ls, i)));
        i = core::Int::add(i, 1);
      }
    }
    operations::reportSplit("a ; b ; c", " ; ");
    operations::reportSplit("def", "");
    operations::reportSplit(" g  h i", " ");
    operations::reportSplit(" ", " ");
    operations::reportSplit("", " ");
    operations::reportSplit("", "");
    operations::reportSplit("🌍", "");
    auto fn11 = [](std::shared_ptr<std::vector<std::string>> list) -> std::shared_ptr<std::vector<std::string>> {
      std::shared_ptr<std::vector<std::string>> builder = core::List::toListBuilder<std::string>(list);
      core::ListBuilder::sort(builder, [](std::string a, std::string b) -> int32_t { return core::Compare::cmp(a, b); });
      return builder;
    };
    std::shared_ptr<std::vector<std::shared_ptr<std::vector<std::string>>>> triangleOfFooBar = core::List::toListBuilder<std::shared_ptr<std::vector<std::string>>>(core::List::map(core::List::make<std::shared_ptr<std::vector<std::string>>>(core::List::make<std::string>(), core::List::make<std::string>("bar"), core::List::make<std::string>("bar", "baz"), core::List::make<std::string>("bar", "baz", "boo"), core::List::make<std::string>("bar", "baz", "boo", "far"), core::List::make<std::string>("bar", "baz", "boo", "far", "faz"), core::List::make<std::string>("far", "faz", "foo", "bar", "baz", "boo")), fn11));
    {
      int32_t j = 0;
      while (j < core::List::length<std::shared_ptr<std::vector<std::string>>>(triangleOfFooBar)) {
        std::shared_ptr<std::vector<std::string>> row = core::List::get<std::shared_ptr<std::vector<std::string>>>(triangleOfFooBar, j);
        core::Console::log(temper_globals::console.value, "Fwd  [" + core::List::join(row, " ", [](std::string x) -> std::string { return x; }) + "]");
        core::ListBuilder::reverse<std::string>(row);
        j = core::Int::add(j, 1);
      }
    }
    core::ListBuilder::reverse<std::shared_ptr<std::vector<std::string>>>(triangleOfFooBar);
    {
      int32_t j2 = 0;
      while (j2 < core::List::length<std::shared_ptr<std::vector<std::string>>>(triangleOfFooBar)) {
        std::shared_ptr<std::vector<std::string>> row2 = core::List::get<std::shared_ptr<std::vector<std::string>>>(triangleOfFooBar, j2);
        core::Console::log(temper_globals::console.value, "Back [" + core::List::join(row2, " ", [](std::string x) -> std::string { return x; }) + "]");
        j2 = core::Int::add(j2, 1);
      }
    }
    std::shared_ptr<std::vector<std::string>> messageBuilder = core::ListBuilder::make<std::string>();
    core::ListBuilder::add<std::string>(messageBuilder, "hello");
    core::ListBuilder::set<std::string>(messageBuilder, 0, "hi");
    core::ListBuilder::addAll<std::string>(messageBuilder, core::List::make<std::string>("there", "y'all"));
    core::ListBuilder::add<std::string>(messageBuilder, "say", 0);
    std::shared_ptr<std::vector<std::string>> copyThat = core::List::toListBuilder<std::string>(messageBuilder);
    core::ListBuilder::addAll<std::string>(messageBuilder, core::List::make<std::string>("more", "than"), 1);
    core::ListBuilder::addAll<std::string>(copyThat, messageBuilder);
    core::Console::log(temper_globals::console.value, core::List::join(messageBuilder, " ", [](std::string x) -> std::string { return x; }));
    core::Console::log(temper_globals::console.value, core::List::join(core::List::sorted(copyThat, [](std::string a, std::string b) -> int32_t { return core::Compare::cmp(b, a); }), " ", [](std::string x) -> std::string { return x; }));
    core::Console::log(temper_globals::console.value, core::List::join(copyThat, " ", [](std::string x) -> std::string { return x; }));
    std::shared_ptr<std::vector<std::string>> typed = messageBuilder;
    std::string oldLast = core::ListBuilder::removeLast<std::string>(copyThat);
    core::Console::log(temper_globals::console.value, "Removed " + oldLast + " so " + core::List::join(copyThat, " ", [](std::string x) -> std::string { return x; }));
    std::shared_ptr<std::vector<std::string>> nextLast = core::ListBuilder::splice<std::string>(copyThat, core::Int::sub(core::List::length<std::string>(copyThat), 1), 1);
    std::shared_ptr<std::vector<std::string>> oldWords = core::ListBuilder::splice<std::string>(copyThat, 4, 3, nextLast);
    core::ListBuilder::splice<std::string>(copyThat, nullptr, 0, oldWords);
    core::ListBuilder::splice<std::string>(copyThat, core::List::length<std::string>(copyThat), nullptr, core::List::make<std::string>("and", "the", "end"));
    core::Console::log(temper_globals::console.value, core::List::join(copyThat, " ", [](std::string x) -> std::string { return x; }));
    core::ListBuilder::splice<std::string>(copyThat, nullptr, nullptr, core::List::make<std::string>("gone"));
    core::Console::log(temper_globals::console.value, core::List::join(copyThat, " ", [](std::string x) -> std::string { return x; }));
    core::ListBuilder::clear<std::string>(messageBuilder);
    core::ListBuilder::add<std::string>(messageBuilder, "Bye");
    core::Console::log(temper_globals::console.value, core::List::join(messageBuilder, " ", [](std::string x) -> std::string { return x; }));
    {
      int32_t i2 = -2;
      while (i2 <= 10) {
        core::Console::log(temper_globals::console.value, "Slice(" + std::to_string(i2) + ", " + std::to_string(core::Int::add(i2, 2)) + "): [" + core::List::join(core::List::slice<int32_t>(ls, i2, core::Int::add(i2, 2)), ", ", [](int32_t x) -> std::string { return std::to_string(x); }) + "]");
        i2 = core::Int::add(i2, 1);
      }
    }
    core::Console::log(temper_globals::console.value, "Outside fallback: " + std::to_string(core::List::getOr<int32_t>(ls, core::List::length<int32_t>(ls), 123)));
    {
      std::function<std::string(bool)> booleanToString = [](bool b) -> std::string { return core::Boolean::toString(b); };
      std::shared_ptr<std::vector<bool>> ls2 = core::List::make<bool>(true, false);
      std::string joined = core::List::join(ls2, ", ", booleanToString);
      std::shared_ptr<std::vector<bool>> inverted = core::List::map(ls2, [](bool b) -> bool { return !b; });
      std::string joinedAndInverted = core::List::join(inverted, ", ", [](bool b) -> std::string { if (b) { return "not false"; } else { return "not true"; } });
      core::Console::log(temper_globals::console.value, "list = [" + joined + "]");
      core::Console::log(temper_globals::console.value, "list (each inverted) = [" + joinedAndInverted + "]");
      std::shared_ptr<std::vector<bool>> onlyTrue = core::List::filter(ls2, [](bool b) -> bool { return b; });
      std::shared_ptr<std::vector<bool>> onlyFalse = core::List::filter(ls2, [](bool b) -> bool { return !b; });
      core::Console::log(temper_globals::console.value, "list (== true only) = [" + core::List::join(onlyTrue, ", ", booleanToString) + "]");
      core::Console::log(temper_globals::console.value, "list (== false only) = [" + core::List::join(onlyFalse, ", ", booleanToString) + "]");
    }
    {
      std::shared_ptr<std::vector<double>> sizes = core::List::make<double>(0.25, 3.0);
      std::shared_ptr<std::vector<std::string>> labels = core::List::map(sizes, [](double f) -> std::string { if (core::Float64::lt(f, 1.0)) { return "small"; } else { return "big"; } });
      core::Console::log(temper_globals::console.value, "sizes = [" + core::List::join(labels, ", ", [](std::string s) -> std::string { return s; }) + "]");
    }
    runSomething();
    {
      std::shared_ptr<std::vector<std::string>> aListBuilder = core::ListBuilder::make<std::string>();
      core::ListBuilder::add<std::string>(aListBuilder, "first");
      std::shared_ptr<std::vector<std::string>> builtFirst = core::List::toList<std::string>(aListBuilder);
      core::ListBuilder::add<std::string>(aListBuilder, "second");
      std::shared_ptr<std::vector<std::string>> builtSecond = core::List::toList<std::string>(aListBuilder);
      core::ListBuilder::add<std::string>(aListBuilder, "third");
      core::Console::log(temper_globals::console.value, "builtFirst has " + core::List::join(builtFirst, ", ", [](std::string s) -> std::string { return s; }));
      core::Console::log(temper_globals::console.value, "builtSecond has " + core::List::join(builtSecond, ", ", [](std::string s) -> std::string { return s; }));
    }
  });
}

}  // namespace operations
