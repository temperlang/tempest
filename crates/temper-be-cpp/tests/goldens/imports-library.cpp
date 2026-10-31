// init.hpp
#pragma once

#include <temper-core/core.hpp>
#include <modules/init.hpp>
#include <modules/helpers.hpp>

namespace imports_library {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

int32_t both(int32_t n);

}  // namespace imports_library

// init.cpp
#include <imports-library/init.hpp>

namespace imports_library {

namespace core = temper::core;

int32_t both(int32_t n) {
  global_init_init();
  int32_t int32 = modules::sum(n);
  return core::Int::add(core::Int::add(int32, modules::twice(n)), modules::temper_globals::base.value);
}

void global_init_init() {
  core::init_once([] {
    modules::global_init_init();
    modules::global_init_helpers();
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    core::Console::log(console, "both " + std::to_string(both(2)));
  });
}

}  // namespace imports_library
