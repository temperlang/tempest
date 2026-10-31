// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace limits {

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace limits

// init.cpp
#include <limits/init.hpp>

#include <mutex>

namespace limits {

namespace core = temper::core;

static std::shared_ptr<core::Console::Type> console;

static void checkParse32(std::string string) {
  std::string converted;
  try {
    converted = std::to_string(core::String::toInt32(string, 16));
  } catch (core::TemperBubble const &) {
    converted = "failed";
  }
  core::Console::log(console, "Parse " + string + " -> " + converted);
}

static void checkParse64(std::string string) {
  std::string converted;
  try {
    converted = std::to_string(core::String::toInt64(string, 16));
  } catch (core::TemperBubble const &) {
    converted = "failed";
  }
  core::Console::log(console, "Parse " + string + " -> " + converted);
}

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    console = core::Console::get_console();
    core::Console::log(console, "Int32");
    int32_t one = 1;
    int32_t big = core::Int::mul(2147483647, one);
    int32_t small = core::Int::mul(INT32_MIN, one);
    core::Console::log(console, std::to_string(big) + " + 1 == " + std::to_string(core::Int::add(big, 1)));
    core::Console::log(console, std::to_string(small) + " - 1 == " + std::to_string(core::Int::sub(small, 1)));
    core::Console::log(console, std::to_string(big) + " * 4 == " + std::to_string(core::Int::mul(big, 4)));
    core::Console::log(console, std::to_string(small) + " * 4 == " + std::to_string(core::Int::mul(small, 4)));
    core::Console::log(console, std::to_string(core::Int::add(small, 1)) + " * 4 == " + std::to_string(core::Int::mul(core::Int::add(small, 1), 4)));
    core::Console::log(console, std::to_string(core::Int::add(small, 1)) + " * " + std::to_string(big) + " == " + std::to_string(core::Int::mul(core::Int::add(small, 1), big)));
    core::Console::log(console, std::to_string(small) + " / -1 == " + std::to_string(core::Int::div_wrap(small, -1)));
    core::Console::log(console, std::to_string(small) + " % -1 == " + std::to_string(core::Int::mod_wrap(small, -1)));
    core::Console::log(console, "-(" + std::to_string(small) + ") == " + std::to_string(core::Int::neg(small)));
    int32_t three = core::Int::mul(3, one);
    int32_t five = core::Int::mul(5, one);
    core::Console::log(console, std::to_string(five) + " / " + std::to_string(three) + " == " + std::to_string(core::Int::div_wrap(five, three)));
    core::Console::log(console, std::to_string(five) + " / -" + std::to_string(three) + " == " + std::to_string(core::Int::div_wrap(five, core::Int::neg(three))));
    core::Console::log(console, "-" + std::to_string(five) + " / " + std::to_string(three) + " == " + std::to_string(core::Int::div_wrap(core::Int::neg(five), three)));
    core::Console::log(console, "-" + std::to_string(five) + " / -" + std::to_string(three) + " == " + std::to_string(core::Int::div_wrap(core::Int::neg(five), core::Int::neg(three))));
    core::Console::log(console, std::to_string(five) + " % " + std::to_string(three) + " == " + std::to_string(core::Int::mod_wrap(five, three)));
    core::Console::log(console, std::to_string(five) + " % -" + std::to_string(three) + " == " + std::to_string(core::Int::mod_wrap(five, core::Int::neg(three))));
    core::Console::log(console, "-" + std::to_string(five) + " % " + std::to_string(three) + " == " + std::to_string(core::Int::mod_wrap(core::Int::neg(five), three)));
    core::Console::log(console, "-" + std::to_string(five) + " % -" + std::to_string(three) + " == " + std::to_string(core::Int::mod_wrap(core::Int::neg(five), core::Int::neg(three))));
    core::Console::log(console, "Int64");
    int64_t notSoBig = core::Int::toInt64(big);
    int64_t notSoSmall = core::Int::toInt64(small);
    core::Console::log(console, std::to_string(notSoBig) + " + 1 == " + std::to_string(core::Int64::add(notSoBig, int64_t{1})));
    core::Console::log(console, std::to_string(notSoSmall) + " - 1 == " + std::to_string(core::Int64::sub(notSoSmall, int64_t{1})));
    int64_t big64 = core::Int64::mul(int64_t{9223372036854775807}, core::Int::toInt64(one));
    int64_t small64 = core::Int64::mul(INT64_MIN, core::Int::toInt64(one));
    core::Console::log(console, std::to_string(big64) + " + 1 == " + std::to_string(core::Int64::add(big64, int64_t{1})));
    core::Console::log(console, std::to_string(small64) + " - 1 == " + std::to_string(core::Int64::sub(small64, int64_t{1})));
    core::Console::log(console, std::to_string(small64) + " / -1 == " + std::to_string(core::Int64::div_wrap(small64, int64_t{-1})));
    core::Console::log(console, std::to_string(small64) + " % -1 == " + std::to_string(core::Int64::mod_wrap(small64, int64_t{-1})));
    core::Console::log(console, "-(" + std::to_string(small64) + ") == " + std::to_string(core::Int64::neg(small64)));
    int64_t three64 = core::Int::toInt64(three);
    int64_t five64 = core::Int::toInt64(five);
    core::Console::log(console, std::to_string(five64) + " / " + std::to_string(three64) + " == " + std::to_string(core::Int64::div_wrap(five64, three64)));
    core::Console::log(console, std::to_string(five64) + " / -" + std::to_string(three64) + " == " + std::to_string(core::Int64::div_wrap(five64, core::Int64::neg(three64))));
    core::Console::log(console, "-" + std::to_string(five64) + " / " + std::to_string(three64) + " == " + std::to_string(core::Int64::div_wrap(core::Int64::neg(five64), three64)));
    core::Console::log(console, "-" + std::to_string(five64) + " / -" + std::to_string(three64) + " == " + std::to_string(core::Int64::div_wrap(core::Int64::neg(five64), core::Int64::neg(three64))));
    core::Console::log(console, std::to_string(five64) + " % " + std::to_string(three64) + " == " + std::to_string(core::Int64::mod_wrap(five64, three64)));
    core::Console::log(console, std::to_string(five64) + " % -" + std::to_string(three64) + " == " + std::to_string(core::Int64::mod_wrap(five64, core::Int64::neg(three64))));
    core::Console::log(console, "-" + std::to_string(five64) + " % " + std::to_string(three64) + " == " + std::to_string(core::Int64::mod_wrap(core::Int64::neg(five64), three64)));
    core::Console::log(console, "-" + std::to_string(five64) + " % -" + std::to_string(three64) + " == " + std::to_string(core::Int64::mod_wrap(core::Int64::neg(five64), core::Int64::neg(three64))));
    checkParse32("7FFFFFFF");
    checkParse32("-80000000");
    checkParse32("80000000");
    checkParse32("-80000001");
    core::Console::log(console, "Not so big: " + std::to_string(core::Int64::toInt32Unsafe(notSoBig)));
    std::string value;
    try {
      value = std::to_string(core::Int64::toInt32(big64));
    } catch (core::TemperBubble const &) {
      value = "no";
    }
    core::Console::log(console, "Actually big: " + value);
    checkParse64("7FFFFFFFFFFFFFFF");
    checkParse64("-8000000000000000");
    checkParse64("8000000000000000");
    checkParse64("-8000000000000001");
    int64_t min = std::min(small64, big64);
    core::Console::log(console, std::to_string(min) + " < " + std::to_string(std::max(small64, big64)));
  });
}

}  // namespace limits
