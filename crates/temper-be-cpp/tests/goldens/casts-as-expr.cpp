// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace as_expr {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

struct Apple;
struct Fuji;
struct Gala;

struct Apple : public virtual temper::core::AnyValueBase {
  virtual std::string get_name() const = 0;
};

struct Fuji : public virtual Apple {
  static std::shared_ptr<Fuji> make();
  std::string get_name() const override;
};

struct Gala : public virtual Apple {
  static std::shared_ptr<Gala> make();
  std::string get_name() const override;
};

// Storage, which the library reads and its init sets; read values
// through the accessors below.
namespace temper_globals {
extern temper::core::GlobalSlot<std::shared_ptr<temper::core::AnyValueBase>> thing0;
extern temper::core::GlobalSlot<std::shared_ptr<temper::core::AnyValueBase>> thing1;
extern temper::core::GlobalSlot<std::shared_ptr<temper::core::AnyValueBase>> thing2;
extern temper::core::GlobalSlot<std::shared_ptr<temper::core::AnyValueBase>> thing3;
}  // namespace temper_globals

std::shared_ptr<temper::core::AnyValueBase> const & thing0();
std::shared_ptr<temper::core::AnyValueBase> const & thing1();
std::shared_ptr<temper::core::AnyValueBase> const & thing2();
std::shared_ptr<temper::core::AnyValueBase> const & thing3();

}  // namespace as_expr

// init.cpp
#include <as-expr/init.hpp>

namespace as_expr {

namespace core = temper::core;

namespace temper_globals {

core::GlobalSlot<std::shared_ptr<core::AnyValueBase>> thing0;
core::GlobalSlot<std::shared_ptr<core::AnyValueBase>> thing1;
core::GlobalSlot<std::shared_ptr<core::AnyValueBase>> thing2;
core::GlobalSlot<std::shared_ptr<core::AnyValueBase>> thing3;

}  // namespace temper_globals

std::shared_ptr<core::AnyValueBase> const & thing0() {
  global_init_init();
  return temper_globals::thing0.value;
}

std::shared_ptr<core::AnyValueBase> const & thing1() {
  global_init_init();
  return temper_globals::thing1.value;
}

std::shared_ptr<core::AnyValueBase> const & thing2() {
  global_init_init();
  return temper_globals::thing2.value;
}

std::shared_ptr<core::AnyValueBase> const & thing3() {
  global_init_init();
  return temper_globals::thing3.value;
}

std::shared_ptr<Fuji> Fuji::make() {
  std::shared_ptr<Fuji> self = std::make_shared<Fuji>();
  return self;
}

std::string Fuji::get_name() const {
  return "Fuji";
}

std::shared_ptr<Gala> Gala::make() {
  std::shared_ptr<Gala> self = std::make_shared<Gala>();
  return self;
}

std::string Gala::get_name() const {
  return "Gala";
}

void global_init_init() {
  core::init_once([] {
    temper_globals::thing0.construct();
    temper_globals::thing1.construct();
    temper_globals::thing2.construct();
    temper_globals::thing3.construct();
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    {
      std::shared_ptr<Apple> x;
      x = Fuji::make();
      core::Console::log(console, core::checked_cast<Apple>(x)->get_name());
      core::Console::log(console, core::checked_cast<Fuji>(x)->get_name());
      x = Gala::make();
      std::string value;
      try {
        value = core::checked_cast<Fuji>(x)->get_name();
      } catch (core::TemperBubble const &) {
        value = "Cast failed!";
      }
      core::Console::log(console, value);
      std::shared_ptr<core::AnyValueBase> y = x;
      std::string value2;
      try {
        value2 = core::checked_cast<Apple>(y)->get_name();
      } catch (core::TemperBubble const &) {
        value2 = "Cast from AnyValue failed!";
      }
      core::Console::log(console, value2);
      std::string value3;
      try {
        value3 = core::checked_cast<Gala>(y)->get_name();
      } catch (core::TemperBubble const &) {
        value3 = "Cast from AnyValue failed!";
      }
      core::Console::log(console, value3);
      std::string value4;
      try {
        value4 = core::checked_cast<Fuji>(y)->get_name();
      } catch (core::TemperBubble const &) {
        value4 = "Cast from AnyValue failed!";
      }
      core::Console::log(console, value4);
      x = nullptr;
      std::string value5;
      try {
        core::checked_cast<Apple>(x);
        value5 = "Tasty nothing.";
      } catch (core::TemperBubble const &) {
        value5 = "Was null!";
      }
      core::Console::log(console, value5);
    }
    temper_globals::thing0.value = core::any_box(false);
    temper_globals::thing1.value = core::any_box(1);
    temper_globals::thing2.value = core::any_box(std::string("two"));
    temper_globals::thing3.value = core::any_box(3.14);
  });
}

}  // namespace as_expr
