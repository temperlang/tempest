// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace hello_world {

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace hello_world

// init.cpp
#include <hello-world/init.hpp>

#include <mutex>

namespace hello_world {

namespace core = temper::core;

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    core::Console::log(console, "Hello, World!");
  });
}

}  // namespace hello_world
