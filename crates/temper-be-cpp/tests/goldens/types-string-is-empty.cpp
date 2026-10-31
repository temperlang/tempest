// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace is_empty {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

}  // namespace is_empty

// init.cpp
#include <is-empty/init.hpp>

namespace is_empty {

namespace core = temper::core;

void global_init_init() {
  core::init_once([] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    std::string emptyString = "";
    std::string foo = "foo";
    core::Console::log(console, emptyString.empty() ? "emptyString is empty" : "emptyString is not empty");
    core::Console::log(console, foo.empty() ? "foo is empty" : "foo is not empty");
  });
}

}  // namespace is_empty
