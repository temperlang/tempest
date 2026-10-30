// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace static_properties_scope {

struct Simple;

struct Simple : public std::enable_shared_from_this<Simple> {
  static std::string value;
  static std::string other;
  static std::shared_ptr<Simple> make();
};

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace static_properties_scope

// init.cpp
#include <static-properties-scope/init.hpp>

#include <mutex>

namespace static_properties_scope {

namespace core = temper::core;

std::string Simple::value;
std::string Simple::other;

std::shared_ptr<Simple> Simple::make() {
  std::shared_ptr<Simple> self = std::make_shared<Simple>();
  return self;
}

static std::string readSimpleValue() {
  return Simple::value;
}

static std::string readSimpleOther() {
  return Simple::other;
}

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    Simple::value = "foo";
    Simple::other = "bar";
    core::Console::log(console, "Simple.value is " + readSimpleValue());
    core::Console::log(console, "Simple.other is " + readSimpleOther());
  });
}

}  // namespace static_properties_scope
