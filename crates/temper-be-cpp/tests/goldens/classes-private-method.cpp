// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace private_method {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

struct C;

struct C : public std::enable_shared_from_this<C> {
  static std::shared_ptr<C> make();
  int32_t helper(int32_t x) const;
  int32_t method1(int32_t x) const;
  int32_t method2(int32_t x) const;
};

}  // namespace private_method

// init.cpp
#include <private-method/init.hpp>

namespace private_method {

namespace core = temper::core;

std::shared_ptr<C> C::make() {
  std::shared_ptr<C> self = std::make_shared<C>();
  return self;
}

int32_t C::helper(int32_t x) const {
  return core::Int::add(x, 1);
}

int32_t C::method1(int32_t x) const {
  std::shared_ptr<C const> self = shared_from_this();
  return self->helper(core::Int::mul(x, 2));
}

int32_t C::method2(int32_t x) const {
  std::shared_ptr<C const> self = shared_from_this();
  return self->helper(core::Int::mul(x, 3));
}

void global_init_init() {
  core::init_once([] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    std::shared_ptr<C> c = C::make();
    core::Console::log(console, "method1(10)=" + std::to_string(c->method1(10)));
    core::Console::log(console, "method2(10)=" + std::to_string(c->method2(10)));
  });
}

}  // namespace private_method
