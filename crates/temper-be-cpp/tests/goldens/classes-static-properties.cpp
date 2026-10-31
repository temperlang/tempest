// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace static_properties {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

struct Simple;
struct C;
struct I;

struct Simple : public std::enable_shared_from_this<Simple> {
  struct temper_statics {
    static temper::core::GlobalSlot<std::shared_ptr<Simple>> simon;
    static temper::core::GlobalSlot<std::string> value;
    static temper::core::GlobalSlot<int32_t> other;
  };
  std::string name;
  static std::shared_ptr<Simple> const & simon();
  static std::string const & value();
  static int32_t const & other();
  static std::shared_ptr<Simple> make(std::string name);
  std::string get_name() const;
};

struct C : public std::enable_shared_from_this<C> {
  static std::shared_ptr<C> make();
  static void sayHi();
  static std::string compose(std::string subject, std::string message);
};

struct I : public virtual temper::core::AnyValueBase {
  static void sayBye();
};

}  // namespace static_properties

// init.cpp
#include <static-properties/init.hpp>

namespace static_properties {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;

}  // namespace temper_globals

core::GlobalSlot<std::shared_ptr<Simple>> Simple::temper_statics::simon;

std::shared_ptr<Simple> const & Simple::simon() {
  global_init_init();
  return Simple::temper_statics::simon.value;
}

core::GlobalSlot<std::string> Simple::temper_statics::value;

std::string const & Simple::value() {
  global_init_init();
  return Simple::temper_statics::value.value;
}

core::GlobalSlot<int32_t> Simple::temper_statics::other;

int32_t const & Simple::other() {
  global_init_init();
  return Simple::temper_statics::other.value;
}

std::shared_ptr<Simple> Simple::make(std::string name) {
  std::shared_ptr<Simple> self = std::make_shared<Simple>();
  self->name = name;
  return self;
}

std::string Simple::get_name() const {
  return this->name;
}

std::shared_ptr<C> C::make() {
  std::shared_ptr<C> self = std::make_shared<C>();
  return self;
}

void C::sayHi() {
  core::Console::log(temper_globals::console.value, C::compose("C", "'hi.'"));
}

std::string C::compose(std::string subject, std::string message) {
  return subject + " says " + message;
}

void I::sayBye() {
  core::Console::log(temper_globals::console.value, "I says 'bye.'");
}

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    Simple::temper_statics::simon.construct();
    Simple::temper_statics::value.construct();
    Simple::temper_statics::other.construct();
    temper_globals::console.value = core::Console::get_console();
    Simple::temper_statics::simon.value = Simple::make("foo");
    Simple::temper_statics::value.value = Simple::temper_statics::simon.value->name;
    Simple::temper_statics::other.value = 22;
    core::Console::log(temper_globals::console.value, "Simple.simon.name is " + Simple::temper_statics::simon.value->name);
    core::Console::log(temper_globals::console.value, "Simple.value is " + Simple::temper_statics::value.value);
    core::Console::log(temper_globals::console.value, "Simple.other is " + std::to_string(Simple::temper_statics::other.value));
    C::sayHi();
    I::sayBye();
  });
}

}  // namespace static_properties
