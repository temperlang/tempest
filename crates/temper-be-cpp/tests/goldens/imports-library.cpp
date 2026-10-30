// init.hpp
#pragma once

#include <temper-core/core.hpp>
#include <modules/init.hpp>
#include <modules/helpers.hpp>

namespace imports_library {

int32_t both(int32_t n);

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace imports_library

// init.cpp
#include <imports-library/init.hpp>

#include <mutex>

namespace imports_library {

namespace core = temper::core;

int32_t both(int32_t n) {
  int32_t int32 = modules::sum(n);
  return core::Int::add(core::Int::add(int32, modules::twice(n)), modules::base);
}

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    modules::global_init_init();
    modules::global_init_helpers();
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    core::Console::log(console, "both " + std::to_string(both(2)));
  });
}

}  // namespace imports_library
