// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace bubble {

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace bubble

// init.cpp
#include <bubble/init.hpp>

#include <mutex>

namespace bubble {

namespace core = temper::core;

static std::shared_ptr<core::Console::Type> console;

static int32_t more(core::NullableParam<int32_t>) {
  core::bubble();
}

static std::string something(core::NullableParam<std::string> x) {
  {
    if (x != nullptr) {
      return x.value;
    } else {
      core::bubble();
    }
  }
}

static void report(core::NullableParam<std::string> x) {
  std::string message;
  try {
    message = something(x);
  } catch (core::TemperBubble const &) {
    message = "null";
  }
  std::string suffix;
  try {
    int32_t value;
    try {
      core::not_null(x);
      value = 1;
    } catch (core::TemperBubble const &) {
      value = 0;
    }
    suffix = std::to_string(more(value));
  } catch (core::TemperBubble const &) {
    suffix = "";
  }
  core::Console::log(console, message + suffix);
}

static int32_t calcRows(int32_t cols, int32_t total) {
  core::Console::log(console, "calcRows(" + std::to_string(cols) + ", " + std::to_string(total) + ")");
  if (core::Int::mod_wrap(total, cols) != 0) {
    core::bubble();
  }
  return core::Int::div_wrap(total, cols);
}

static int32_t countBelow(std::string s) {
  int32_t i = 0;
  while (i < core::String::toInt32(s)) {
    i = core::Int::add(i, 1);
  }
  return i;
}

static void branch(int32_t a, int32_t b) {
  std::string c = a == 0 ? (b == 0 ? "00" : b == 1 ? "01" : b == 2 ? "02" : "0x") : a == 1 ? "1x" : a == 2 ? "2x" : a == 3 ? "3x" : "xx";
  core::Console::log(console, "branch(" + std::to_string(a) + ", " + std::to_string(b) + ") -> " + c);
}

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    console = core::Console::get_console();
    report("hi");
    report(nullptr);
    core::Console::log(console, std::to_string(calcRows(3, 6)));
    int32_t value;
    try {
      value = countBelow("abc");
    } catch (core::TemperBubble const &) {
      value = -1;
    }
    core::Console::log(console, std::to_string(value));
    branch(0, 2);
    branch(2, 0);
  });
}

}  // namespace bubble
