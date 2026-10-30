// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace pure_virtual {

struct Greeter;
struct HiGreeter;

struct Greeter : public virtual temper::core::AnyValueBase {
  virtual void greet() const = 0;
};

struct HiGreeter : public virtual Greeter {
  static std::shared_ptr<HiGreeter> make();
  void greet() const override;
};

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace pure_virtual

// init.cpp
#include <pure-virtual/init.hpp>

#include <mutex>

namespace pure_virtual {

namespace core = temper::core;

static std::shared_ptr<core::Console::Type> console;

std::shared_ptr<HiGreeter> HiGreeter::make() {
  std::shared_ptr<HiGreeter> self = std::make_shared<HiGreeter>();
  return self;
}

void HiGreeter::greet() const {
  core::Console::log(console, "Hi, virtually!");
}

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    console = core::Console::get_console();
    std::shared_ptr<Greeter> greeter = HiGreeter::make();
    greeter->greet();
  });
}

}  // namespace pure_virtual
