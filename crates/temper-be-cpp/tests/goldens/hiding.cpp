// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace hiding {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

}  // namespace hiding

// init.cpp
#include <hiding/init.hpp>

namespace hiding {

namespace core = temper::core;

static int32_t explicit_(int32_t int32_t_) {
  return core::Int::add(int32_t_, 1);
}

static int32_t core_(int32_t a, int32_t b) {
  return core::Int::div_wrap(a, b);
}

static bool before(bool a, bool b) {
  return a < b;
}

static std::string shifts(int32_t n) {
  return std::to_string(core::Int::shl(1, n)) + " " + std::to_string(core::Int::shr(-8, n)) + " " + std::to_string(core::Int::ushr(-8, core::Int::sub(n, 4)));
}

void global_init_init() {
  core::init_once([] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    std::string std_ = "s";
    int32_t M_PI_ = 3;
    core::Console::log(console, std_ + " " + std::to_string(explicit_(1)) + " " + std::to_string(core_(7, 2)) + " " + std::to_string(M_PI_));
    core::Console::log(console, core::Boolean::toString(before(false, true)) + " " + core::Boolean::toString(std::string("?\?=") < "?\?>"));
    core::Console::log(console, shifts(33));
    core::Console::log(console, core::Boolean::toString(std::string("a\000b", 3) == std::string("a\000b", 3)) + " " + core::Boolean::toString(std::string("a\000b", 3) == std::string("a\000c", 3)));
  });
}

}  // namespace hiding
