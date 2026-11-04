// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace nonascii {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

}  // namespace nonascii

// init.cpp
#include <nonascii/init.hpp>

namespace nonascii {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;

}  // namespace temper_globals

static void sayHello(std::string όνομα) {
  core::Console::log(temper_globals::console.value, "Hello, " + όνομα + "!");
}

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::console.value = core::Console::get_console();
    nonascii::sayHello("World");
  });
}

}  // namespace nonascii
