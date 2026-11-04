// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace string_order {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

}  // namespace string_order

// init.cpp
#include <string-order/init.hpp>

namespace string_order {

namespace core = temper::core;

static std::string show(std::string a, std::string b) {
  return std::to_string(core::Compare::cmp(a, b)) + " " + core::Boolean::toString(a < b) + " " + core::Boolean::toString(a <= b) + " " + core::Boolean::toString(a > b) + " " + core::Boolean::toString(a >= b);
}

static int32_t cmp(double a, double b) {
  return core::Float64::cmp(a, b);
}

void global_init_init() {
  core::init_once([] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    core::Console::log(console, string_order::show("｡", "😀"));
    core::Console::log(console, string_order::show("😀", "｡"));
    core::Console::log(console, string_order::show("\uE000", "𐀀"));
    core::Console::log(console, string_order::show("\uFFFF", "\U0010FFFF"));
    core::Console::log(console, string_order::show("a｡b", "a😀"));
    core::Console::log(console, string_order::show("😀", "😁"));
    core::Console::log(console, string_order::show("😀", "😀"));
    core::Console::log(console, string_order::show("😀", "😀a"));
    core::Console::log(console, string_order::show("", "😀"));
    core::Console::log(console, string_order::show("", ""));
    core::Console::log(console, string_order::show("z", "a"));
    core::Console::log(console, string_order::show("abc", "abd"));
    core::Console::log(console, std::to_string(cmp(2.5, 1.0)) + " " + std::to_string(cmp(1.0, 2.5)) + " " + std::to_string(cmp(1.5, 1.5)));
  });
}

}  // namespace string_order
