// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace operators {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

}  // namespace operators

// init.cpp
#include <operators/init.hpp>

namespace operators {

namespace core = temper::core;

static std::string same(bool a, bool b) {
  return core::Boolean::toString(a == b) + " " + core::Boolean::toString(a != b) + " " + std::to_string(core::Compare::cmp(a, b));
}

static int32_t quot(int32_t a, int32_t b) {
  return core::Int::div_wrap(a, b);
}

static int32_t rem(int32_t a, int32_t b) {
  return core::Int::mod_wrap(a, b);
}

static std::string words(std::string a, std::string b) {
  if (a < b) {
    return a + " < " + b;
  }
  if (a <= b) {
    return a + " == " + b;
  } else {
    return a + " > " + b;
  }
}

void global_init_init() {
  core::init_once([] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    int32_t one = 1;
    int32_t big = core::Int::mul(2147483647, one);
    int32_t small = core::Int::mul(INT32_MIN, one);
    core::Console::log(console, std::to_string(core::Int::add(big, 1)) + " " + std::to_string(core::Int::sub(small, 1)) + " " + std::to_string(core::Int::mul(big, 4)) + " " + std::to_string(core::Int::neg(small)));
    core::Console::log(console, std::to_string(core::Int::mul(-7, one) >> 1) + " " + std::to_string(core::Int::ushr(core::Int::mul(-7, one), 28)) + " " + std::to_string(core::Int::shl(one, 33)) + " " + std::to_string(~one) + " " + std::to_string(5 & 3) + " " + std::to_string(5 | 3) + " " + std::to_string(5 ^ 3));
    core::Console::log(console, std::to_string(core::Compare::cmp(3, 5)) + " " + std::to_string(core::Compare::cmp<std::string>("b", "a")));
    core::Console::log(console, same(true, true) + " " + same(true, false) + " " + same(false, true));
    core::Console::log(console, std::to_string(quot(-7, 2)) + " " + std::to_string(rem(-7, 2)) + " " + std::to_string(quot(small, -1)));
    core::Console::log(console, operators::words("apple", "banana"));
    core::Console::log(console, operators::words("pear", "pear"));
    core::Console::log(console, operators::words("zoo", "yak"));
    core::Console::log(console, core::Boolean::toString(operators::words("a", "b") == "a < b"));
    int32_t n = 0;
    int32_t odd = 0;
    do {
      n = core::Int::add(n, 1);
      if (n % 2 != 0) {
        odd = core::Int::add(odd, n);
      }
    } while (n < 9);
    core::Console::log(console, "odd " + std::to_string(odd));
    int32_t found = -1;
    int32_t i = 0;
    while (i < 5 && found < 0) {
      int32_t j = 0;
      while (j < 5) {
        if (core::Int::mul(i, j) == 6) {
          found = core::Int::add(core::Int::mul(i, 10), j);
          break;
        }
        j = core::Int::add(j, 1);
      }
      i = core::Int::add(i, 1);
    }
    core::Console::log(console, "found " + std::to_string(found));
    std::string empty = "";
    std::string x = "x";
    core::Console::log(console, core::Boolean::toString(empty.empty() && !x.empty()));
  });
}

}  // namespace operators
