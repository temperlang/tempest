// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace bubble {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

}  // namespace bubble

// init.cpp
#include <bubble/init.hpp>

namespace bubble {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;

}  // namespace temper_globals

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
    message = bubble::something(x);
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
    suffix = std::to_string(bubble::more(value));
  } catch (core::TemperBubble const &) {
    suffix = "";
  }
  core::Console::log(temper_globals::console.value, message + suffix);
}

static int32_t calcRows(int32_t cols, int32_t total) {
  core::Console::log(temper_globals::console.value, "calcRows(" + std::to_string(cols) + ", " + std::to_string(total) + ")");
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

static int32_t firstOver(int32_t n) {
  int32_t i = 0;
  while (i < 9) {
    i = core::Int::add(i, 1);
    if (i > n) {
      break;
    }
  }
  return i;
}

static int32_t sumSkipping(int32_t n) {
  int32_t sum = 0;
  {
    int32_t i = 0;
    while (i < 5) {
      if (i == n) {
        {
          i = core::Int::add(i, 1);
          continue;
        }
      }
      sum = core::Int::add(sum, i);
      i = core::Int::add(i, 1);
    }
  }
  return sum;
}

static int32_t tenTimes(int32_t n) {
  {
    int32_t i = 0;
    while (i < 9) {
      if (i == n) {
        return core::Int::mul(i, 10);
      }
      i = core::Int::add(i, 1);
    }
  }
  core::bubble();
}

static bool isNone(core::NullableParam<int32_t> r) {
  return r == nullptr;
}

static std::string viaOrElse(int32_t n) {
  int32_t value;
  try {
    value = firstOver(n);
  } catch (core::TemperBubble const &) {
    value = -1;
  }
  int32_t value2;
  try {
    value2 = sumSkipping(n);
  } catch (core::TemperBubble const &) {
    value2 = -1;
  }
  int32_t value3;
  try {
    value3 = tenTimes(n);
  } catch (core::TemperBubble const &) {
    value3 = -1;
  }
  return std::to_string(value) + " " + std::to_string(value2) + " " + std::to_string(value3);
}

static void branch(int32_t a, int32_t b) {
  std::string c = a == 0 ? (b == 0 ? "00" : b == 1 ? "01" : b == 2 ? "02" : "0x") : a == 1 ? "1x" : a == 2 ? "2x" : a == 3 ? "3x" : "xx";
  core::Console::log(temper_globals::console.value, "branch(" + std::to_string(a) + ", " + std::to_string(b) + ") -> " + c);
}

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::console.value = core::Console::get_console();
    bubble::report("hi");
    bubble::report(nullptr);
    core::Console::log(temper_globals::console.value, std::to_string(calcRows(3, 6)));
    int32_t value;
    try {
      value = bubble::countBelow("abc");
    } catch (core::TemperBubble const &) {
      value = -1;
    }
    core::Console::log(temper_globals::console.value, std::to_string(value));
    int32_t value2;
    try {
      value2 = firstOver(1);
    } catch (core::TemperBubble const &) {
      value2 = -1;
    }
    core::NullableParam<int32_t> value3;
    try {
      value3 = firstOver(1);
    } catch (core::TemperBubble const &) {
      value3 = nullptr;
    }
    core::Console::log(temper_globals::console.value, std::to_string(value2) + " " + core::Boolean::toString(bubble::isNone(value3)));
    core::Console::log(temper_globals::console.value, viaOrElse(2));
    core::Console::log(temper_globals::console.value, viaOrElse(12));
    branch(0, 2);
    branch(2, 0);
  });
}

}  // namespace bubble
