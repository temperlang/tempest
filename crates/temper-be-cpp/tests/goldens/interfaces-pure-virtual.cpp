// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace pure_virtual {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

struct Greeter;
struct HiGreeter;

struct Greeter : public virtual temper::core::AnyValueBase {
  virtual void greet() const = 0;
};

struct HiGreeter : public virtual Greeter {
  static std::shared_ptr<HiGreeter> make();
  void greet() const override;
};

}  // namespace pure_virtual

// init.cpp
#include <pure-virtual/init.hpp>

namespace pure_virtual {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;

}  // namespace temper_globals

std::shared_ptr<HiGreeter> HiGreeter::make() {
  std::shared_ptr<HiGreeter> self = std::make_shared<HiGreeter>();
  return self;
}

void HiGreeter::greet() const {
  core::Console::log(temper_globals::console.value, "Hi, virtually!");
}

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::console.value = core::Console::get_console();
    std::shared_ptr<Greeter> greeter = HiGreeter::make();
    greeter->greet();
  });
}

}  // namespace pure_virtual
