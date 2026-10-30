// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace if_return {

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace if_return

// init.cpp
#include <if-return/init.hpp>

#include <mutex>

namespace if_return {

namespace core = temper::core;

static std::string explicit_(bool b) {
  if (b) {
    return "true";
  }
  return "false";
}

static std::string halfAndHalf(bool b) {
  if (b) {
    return "true";
  }
  return "false";
}

static std::string implicit(bool b) {
  if (b) {
    return "true";
  } else {
    return "false";
  }
}

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    bool t = true;
    bool f = !t;
    core::Console::log(console, "explicit    true  -> " + explicit_(t));
    core::Console::log(console, "explicit    false -> " + explicit_(f));
    core::Console::log(console, "halfAndHalf true  -> " + halfAndHalf(t));
    core::Console::log(console, "halfAndHalf false -> " + halfAndHalf(f));
    core::Console::log(console, "implicit    true  -> " + implicit(t));
    core::Console::log(console, "implicit    false -> " + implicit(f));
  });
}

}  // namespace if_return
