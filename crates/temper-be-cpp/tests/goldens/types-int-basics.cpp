// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace basics {

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace basics

// init.cpp
#include <basics/init.hpp>

#include <mutex>

namespace basics {

namespace core = temper::core;

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    core::Console::log(console, "1.min(3)");
    core::Console::log(console, std::to_string(std::min(1, 3)));
    core::Console::log(console, "3.min(1)");
    core::Console::log(console, std::to_string(std::min(3, 1)));
    core::Console::log(console, "1.max(3)");
    core::Console::log(console, std::to_string(std::max(1, 3)));
    core::Console::log(console, "3.max(1)");
    core::Console::log(console, std::to_string(std::max(3, 1)));
  });
}

}  // namespace basics
