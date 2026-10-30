// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace nonascii {

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace nonascii

// init.cpp
#include <nonascii/init.hpp>

#include <mutex>

namespace nonascii {

namespace core = temper::core;

static std::shared_ptr<core::Console::Type> console;

static void sayHello(std::string όνομα) {
  core::Console::log(console, "Hello, " + όνομα + "!");
}

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    console = core::Console::get_console();
    sayHello("World");
  });
}

}  // namespace nonascii
