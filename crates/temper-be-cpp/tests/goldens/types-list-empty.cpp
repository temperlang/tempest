// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace types_list_empty {

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace types_list_empty

// init.cpp
#include <types-list-empty/init.hpp>

#include <mutex>

namespace types_list_empty {

namespace core = temper::core;

static std::shared_ptr<core::Console::Type> console;

static void f(std::shared_ptr<std::vector<int32_t>> const & b) {
  if (core::List::isEmpty<int32_t>(b)) {
    core::Console::log(console, "empty");
  } else {
    core::Console::log(console, "not empty");
  }
}

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    console = core::Console::get_console();
    f(core::List::make<int32_t>(2, 3));
    f(core::List::make<int32_t>());
  });
}

}  // namespace types_list_empty
