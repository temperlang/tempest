// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace empty {

struct Greeter;
struct HiGreeter;

struct Greeter : public virtual temper::core::AnyValueBase {};

struct HiGreeter : public virtual Greeter {
  static std::shared_ptr<HiGreeter> make();
  void greet() const;
};

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace empty

// init.cpp
#include <empty/init.hpp>

#include <mutex>

namespace empty {

namespace core = temper::core;

static std::shared_ptr<core::Console::Type> console;

std::shared_ptr<HiGreeter> HiGreeter::make() {
  std::shared_ptr<HiGreeter> self = std::make_shared<HiGreeter>();
  return self;
}

void HiGreeter::greet() const {
  core::Console::log(console, "Hi!");
}

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    console = core::Console::get_console();
    std::shared_ptr<Greeter> greeter = HiGreeter::make();
    core::checked_cast<HiGreeter>(greeter)->greet();
  });
}

}  // namespace empty
