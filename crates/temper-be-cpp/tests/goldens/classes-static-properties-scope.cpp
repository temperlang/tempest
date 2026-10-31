// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace static_properties_scope {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

struct Simple;

struct Simple : public std::enable_shared_from_this<Simple> {
  struct temper_statics {
    static temper::core::GlobalSlot<std::string> value;
    static temper::core::GlobalSlot<std::string> other;
  };
  static std::string const & value();
  static std::string const & other();
  static std::shared_ptr<Simple> make();
};

}  // namespace static_properties_scope

// init.cpp
#include <static-properties-scope/init.hpp>

namespace static_properties_scope {

namespace core = temper::core;

core::GlobalSlot<std::string> Simple::temper_statics::value;

std::string const & Simple::value() {
  global_init_init();
  return Simple::temper_statics::value.value;
}

core::GlobalSlot<std::string> Simple::temper_statics::other;

std::string const & Simple::other() {
  global_init_init();
  return Simple::temper_statics::other.value;
}

std::shared_ptr<Simple> Simple::make() {
  std::shared_ptr<Simple> self = std::make_shared<Simple>();
  return self;
}

static std::string readSimpleValue() {
  return Simple::temper_statics::value.value;
}

static std::string readSimpleOther() {
  return Simple::temper_statics::other.value;
}

void global_init_init() {
  core::init_once([] {
    Simple::temper_statics::value.construct();
    Simple::temper_statics::other.construct();
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    Simple::temper_statics::value.value = "foo";
    Simple::temper_statics::other.value = "bar";
    core::Console::log(console, "Simple.value is " + readSimpleValue());
    core::Console::log(console, "Simple.other is " + readSimpleOther());
  });
}

}  // namespace static_properties_scope
