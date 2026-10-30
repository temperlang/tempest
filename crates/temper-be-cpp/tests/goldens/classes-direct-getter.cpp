// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace direct_getter {

struct C;

struct C : public std::enable_shared_from_this<C> {
  static std::shared_ptr<C> make();
  std::string get_place() const;
};

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace direct_getter

// init.cpp
#include <direct-getter/init.hpp>

#include <mutex>

namespace direct_getter {

namespace core = temper::core;

std::shared_ptr<C> C::make() {
  std::shared_ptr<C> self = std::make_shared<C>();
  return self;
}

std::string C::get_place() const {
  return "Hilo, HI";
}

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    std::shared_ptr<C> c = C::make();
    core::Console::log(console, "Hello, " + c->get_place() + "!");
  });
}

}  // namespace direct_getter
