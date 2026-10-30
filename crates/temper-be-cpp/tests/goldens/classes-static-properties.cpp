// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace static_properties {

struct Simple;
struct C;
struct I;

struct Simple : public std::enable_shared_from_this<Simple> {
  std::string name;
  static std::shared_ptr<Simple> simon;
  static std::string value;
  static int32_t other;
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

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace static_properties

// init.cpp
#include <static-properties/init.hpp>

#include <mutex>

namespace static_properties {

namespace core = temper::core;

static std::shared_ptr<core::Console::Type> console;
std::shared_ptr<Simple> Simple::simon;
std::string Simple::value;
int32_t Simple::other;

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
  core::Console::log(console, C::compose("C", "'hi.'"));
}

std::string C::compose(std::string subject, std::string message) {
  return subject + " says " + message;
}

void I::sayBye() {
  core::Console::log(console, "I says 'bye.'");
}

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    console = core::Console::get_console();
    Simple::simon = Simple::make("foo");
    Simple::value = Simple::simon->name;
    Simple::other = 22;
    core::Console::log(console, "Simple.simon.name is " + Simple::simon->name);
    core::Console::log(console, "Simple.value is " + Simple::value);
    core::Console::log(console, "Simple.other is " + std::to_string(Simple::other));
    C::sayHi();
    I::sayBye();
  });
}

}  // namespace static_properties
