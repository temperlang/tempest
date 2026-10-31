// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace types_float_basics {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

}  // namespace types_float_basics

// init.cpp
#include <types-float-basics/init.hpp>

namespace types_float_basics {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;

}  // namespace temper_globals

static bool gt(double a, double b) {
  return core::Float64::gt(a, b);
}

static bool lt(double a, double b) {
  return core::Float64::lt(a, b);
}

static bool eq(double a, double b) {
  return core::Float64::eq(a, b);
}

static bool neq(double a, double b) {
  return !core::Float64::eq(a, b);
}

static double add(double a, double b) {
  return a + b;
}

static void checkParse(std::string string) {
  std::string converted;
  try {
    converted = core::Float64::toString(core::String::toFloat64(string));
  } catch (core::TemperBubble const &) {
    converted = "failed";
  }
  core::Console::log(temper_globals::console.value, "Parse " + string + " -> " + converted);
}

static void checkTiny(std::string string) {
  std::string positive;
  try {
    positive = core::Boolean::toString(core::Float64::gt(core::String::toFloat64(string), 0.0));
  } catch (core::TemperBubble const &) {
    positive = "failed";
  }
  core::Console::log(temper_globals::console.value, "Parse " + string + " > 0 -> " + positive);
}

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::console.value = core::Console::get_console();
    double negativeZero = -0.0;
    double zero = 0.0;
    double one = 1.0;
    double two = 2.0;
    double three = 3.0;
    if (gt(one, zero)) {
      core::Console::log(temper_globals::console.value, "greater than");
    }
    if (lt(zero, one)) {
      core::Console::log(temper_globals::console.value, "less than");
    }
    if (eq(zero, one)) {
      core::Console::log(temper_globals::console.value, "equality is wrong");
    }
    if (neq(zero, one)) {
      core::Console::log(temper_globals::console.value, "inequality");
    }
    core::Console::log(temper_globals::console.value, core::Float64::toString(zero));
    if (neq(negativeZero, zero)) {
      core::Console::log(temper_globals::console.value, "zeroes not equal");
    }
    if (lt(negativeZero, zero)) {
      core::Console::log(temper_globals::console.value, "negZero < zero");
    }
    if (core::Float64::eq(zero - zero, zero)) {
      core::Console::log(temper_globals::console.value, "zero subtraction");
    }
    if (core::Float64::eq(zero + zero, zero)) {
      core::Console::log(temper_globals::console.value, "zero addition");
    }
    if (core::Float64::eq(negativeZero - zero, negativeZero)) {
      core::Console::log(temper_globals::console.value, "negative zero minus zero is negative");
    }
    if (core::Float64::eq(negativeZero + zero, zero)) {
      core::Console::log(temper_globals::console.value, "negative zero plus zero is positive");
    }
    if (core::Float64::eq(zero - negativeZero, zero)) {
      core::Console::log(temper_globals::console.value, "zero minus negative zero is zero");
    }
    if (core::Float64::eq(zero + negativeZero, zero)) {
      core::Console::log(temper_globals::console.value, "zero plus negative zero is zero");
    }
    if (core::Float64::eq(negativeZero - negativeZero, zero)) {
      core::Console::log(temper_globals::console.value, "negative zero minus negative zero is zero");
    }
    if (core::Float64::eq(negativeZero + negativeZero, negativeZero)) {
      core::Console::log(temper_globals::console.value, "negative zero plus negative zero is negative zero");
    }
    core::Console::log(temper_globals::console.value, core::Float64::toString(negativeZero));
    double third = one / three;
    double half = one / two;
    double twoThirds = two / three;
    if (gt(third, half)) {
      core::Console::log(temper_globals::console.value, "oops third > half");
    }
    if (gt(half, twoThirds)) {
      core::Console::log(temper_globals::console.value, "oops half > twoThirds");
    }
    if (neq(third * two, twoThirds)) {
      core::Console::log(temper_globals::console.value, "misaligned multiplication");
    }
    if (neq(add(third, twoThirds), one)) {
      core::Console::log(temper_globals::console.value, "doesn't add back up");
    }
    core::Console::log(temper_globals::console.value, core::Float64::toString(half));
    core::Console::log(temper_globals::console.value, core::Float64::toString(core::Float64::pow(4.0, -half)));
    core::Console::log(temper_globals::console.value, core::Float64::toString(third));
    core::Console::log(temper_globals::console.value, core::Float64::toString(twoThirds));
    core::Console::log(temper_globals::console.value, core::Float64::toString(one));
    double almostBig = 1e25 * one;
    core::Console::log(temper_globals::console.value, core::Float64::toString(almostBig));
    double big = 99999999999999.0 * 999999999999.0;
    core::Console::log(temper_globals::console.value, core::Float64::toString(big));
    double bigger = big * big * big * big * big * big * big * big;
    core::Console::log(temper_globals::console.value, core::Float64::toString(bigger));
    double biggest = bigger * bigger * bigger * bigger * bigger * bigger;
    core::Console::log(temper_globals::console.value, "biggest " + core::Float64::toString(biggest));
    double tiniest = one / biggest;
    core::Console::log(temper_globals::console.value, "tiniest " + core::Float64::toString(tiniest));
    double negativeInfinity = biggest * -1.0;
    core::Console::log(temper_globals::console.value, "negative infinity " + core::Float64::toString(negativeInfinity));
    double infinity = std::numeric_limits<double>::infinity();
    core::Console::log(temper_globals::console.value, core::Float64::toString(infinity));
    core::Console::log(temper_globals::console.value, "processed an infinity");
    double nan = std::numeric_limits<double>::quiet_NaN();
    core::Console::log(temper_globals::console.value, core::Float64::toString(nan));
    if (core::Float64::gt(nan, infinity)) {
      core::Console::log(temper_globals::console.value, "NaN is biggest");
    } else {
      core::Console::log(temper_globals::console.value, "Infinity is biggest");
    }
    core::Console::log(temper_globals::console.value, "NaN == NaN? " + core::Boolean::toString(core::Float64::eq(nan, std::numeric_limits<double>::quiet_NaN())));
    core::Console::log(temper_globals::console.value, "Int to float: " + core::Float64::toString(core::Int::toFloat64(core::Float64::toInt32Unsafe(two))));
    core::Console::log(temper_globals::console.value, "Positive to int: " + std::to_string(core::Float64::toInt32Unsafe(4.9 * one)));
    core::Console::log(temper_globals::console.value, "Negative to int: " + std::to_string(core::Float64::toInt32Unsafe(-4.9 * one)));
    std::string value;
    try {
      value = std::to_string(core::Float64::toInt32(infinity));
    } catch (core::TemperBubble const &) {
      value = "no";
    }
    core::Console::log(temper_globals::console.value, "Infinity to int: " + value);
    std::string value2;
    try {
      value2 = std::to_string(core::Float64::toInt32(std::numeric_limits<double>::quiet_NaN()));
    } catch (core::TemperBubble const &) {
      value2 = "no";
    }
    core::Console::log(temper_globals::console.value, "NaN to int: " + value2);
    int64_t oneInt64 = core::Float64::toInt64Unsafe(one);
    int64_t notSoBig = core::Int64::mul(int64_t{2147483647}, oneInt64);
    int64_t notSoSmall = core::Int64::mul(int64_t{-2147483648}, oneInt64);
    int64_t slightlyBigger = core::Float64::toInt64Unsafe(core::Int64::toFloat64Unsafe(notSoBig) + 1.5);
    int64_t slightlySmaller = core::Float64::toInt64Unsafe(core::Int64::toFloat64Unsafe(notSoSmall) - 1.5);
    int64_t tooBigForFloat64 = core::Int64::mul(int64_t{9007199254740992}, core::Float64::toInt64(one));
    double okToConvert64 = core::Int64::toFloat64Unsafe(core::Int64::sub(tooBigForFloat64, int64_t{1}));
    double sneakyBig = okToConvert64 + 1.0;
    core::Console::log(temper_globals::console.value, "Bigger to Int64: " + std::to_string(slightlyBigger));
    core::Console::log(temper_globals::console.value, "Smaller to Int64: " + std::to_string(slightlySmaller));
    core::Console::log(temper_globals::console.value, "Barely ok to Int64: " + std::to_string(core::Float64::toInt64Unsafe(okToConvert64)));
    std::string value3;
    try {
      value3 = std::to_string(core::Float64::toInt64(sneakyBig));
    } catch (core::TemperBubble const &) {
      value3 = "no";
    }
    core::Console::log(temper_globals::console.value, "Sneaky big to Int64: " + value3);
    std::string value4;
    try {
      value4 = core::Float64::toString(core::Int64::toFloat64(tooBigForFloat64));
    } catch (core::TemperBubble const &) {
      value4 = "no";
    }
    core::Console::log(temper_globals::console.value, "Too big Int64 to Float64: " + value4);
    std::string value5;
    try {
      value5 = std::to_string(core::Float64::toInt64(infinity));
    } catch (core::TemperBubble const &) {
      value5 = "no";
    }
    core::Console::log(temper_globals::console.value, "Infinity to Int64: " + value5);
    std::string value6;
    try {
      value6 = std::to_string(core::Float64::toInt64(std::numeric_limits<double>::quiet_NaN()));
    } catch (core::TemperBubble const &) {
      value6 = "no";
    }
    core::Console::log(temper_globals::console.value, "NaN to Int64: " + value6);
    checkParse("-1.5");
    checkParse("-0");
    checkParse("2.0");
    checkParse("2");
    checkParse(" 2 ");
    checkParse("5e-1");
    checkParse("NaN");
    checkParse("-Infinity");
    checkTiny("2.225e-308");
    checkTiny("5e-324");
    checkParse("1e-400");
    checkParse("-1e-400");
    checkParse("2.");
    checkParse(".2");
    checkParse("2.0.0");
    checkParse("-inf");
    checkParse("totally");
  });
}

}  // namespace types_float_basics
