// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace ops {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

}  // namespace ops

// init.cpp
#include <ops/init.hpp>

namespace ops {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;
static core::GlobalSlot<double> two;

}  // namespace temper_globals

static void near(std::string label, double actual, double expected) {
  bool subject = core::Float64::near(actual, expected);
  std::string result = subject == true ? "✅" : "❌ wanted " + core::Float64::toString(expected) + ", was " + core::Float64::toString(actual);
  core::Console::log(temper_globals::console.value, label + ": " + result);
}

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::two.construct();
    temper_globals::console.value = core::Console::get_console();
    double one = 1.0;
    temper_globals::two.value = one + one;
    double half = one / temper_globals::two.value;
    near("half.abs()", core::Float64::abs(half), half);
    near("(-half).abs()", core::Float64::abs(-half), half);
    double three = one + temper_globals::two.value;
    near("half.acos()", core::Float64::acos(half), core::Float64::pi() / three);
    double pi = core::Float64::pi();
    double six = temper_globals::two.value * three;
    near("half.asin()", core::Float64::asin(half), pi / six);
    double four = temper_globals::two.value * temper_globals::two.value;
    near("(-one).atan()", core::Float64::atan(-one), -pi / four);
    near("one.atan2(-one)", core::Float64::atan2(one, -one), (three * pi) / four);
    near("(four / three).ceil()", core::Float64::ceil(four / three), temper_globals::two.value);
    near("(-four / three).ceil()", core::Float64::ceil(-four / three), -one);
    near("(pi / three).cos()", core::Float64::cos(pi / three), half);
    double e = core::Float64::e();
    near("one.cosh()", core::Float64::cosh(one), (e + one / e) / temper_globals::two.value);
    near("half.exp() == e.sqrt()", core::Float64::exp(half), core::Float64::sqrt(e));
    near("half.expm1()", core::Float64::expm1(half), core::Float64::sqrt(e) - one);
    near("(four / three).floor()", core::Float64::floor(four / three), one);
    near("(-four / three).floor()", core::Float64::floor(-four / three), -temper_globals::two.value);
    near("e.log()", core::Float64::log(core::Float64::e()), one);
    double five = one + four;
    double ten = temper_globals::two.value * five;
    near("ten.log10()", core::Float64::log10(ten), one);
    near("(e - one).log1p()", core::Float64::log1p(e - one), one);
    auto fn = [](double x) -> std::string {
      auto fn2 = [=](double y) -> std::string {
        std::string pair = core::List::join(core::List::make<double>(core::Float64::max(x, y), core::Float64::min(x, y)), ", ", [](double z) -> std::string { return core::Float64::toString(z); });
        return "(" + pair + ")";
      };
      return core::List::join(core::List::make<double>(std::numeric_limits<double>::quiet_NaN(), temper_globals::two.value), ", ", fn2);
    };
    std::string maxMinCombos = core::List::join(core::List::make<double>(std::numeric_limits<double>::quiet_NaN(), one), ", ", fn);
    core::Console::log(temper_globals::console.value, "maxMinCombos: " + maxMinCombos);
    double quarter = half / temper_globals::two.value;
    near("2.25 % 2.0", std::fmod(temper_globals::two.value + quarter, temper_globals::two.value), 0.25);
    near("2.25 % -2.0", std::fmod(temper_globals::two.value + quarter, -temper_globals::two.value), 0.25);
    near("-2.25 % 2.0", std::fmod(-(temper_globals::two.value + quarter), temper_globals::two.value), -0.25);
    near("-2.25 % -2.0", std::fmod(-(temper_globals::two.value + quarter), -temper_globals::two.value), -0.25);
    near("pi.round()", core::Float64::round(pi), three);
    near("(-pi).round()", core::Float64::round(-pi), -three);
    near("pi.sign()", core::Float64::sign(pi), one);
    near("(-pi).sign()", core::Float64::sign(-pi), -one);
    double zero = one - one;
    core::Console::log(temper_globals::console.value, "zero.sign(): " + core::Float64::toString(core::Float64::sign(zero)));
    core::Console::log(temper_globals::console.value, "(-zero).sign(): " + core::Float64::toString(core::Float64::sign(-zero)));
    near("(pi / six).sin()", core::Float64::sin(pi / six), half);
    near("one.sinh()", core::Float64::sinh(one), (e - one / e) / temper_globals::two.value);
    near("(-pi / four).tan()", core::Float64::tan(-pi / four), -one);
    near("(three * pi / four).tan()", core::Float64::tan((three * pi) / four), -one);
    near("one.tanh()", core::Float64::tanh(one), (core::Float64::pow(e, 2.0) - 1.0) / (core::Float64::pow(e, 2.0) + 1.0));
    core::Console::log(temper_globals::console.value, "one.near(one + 0.1): " + core::Boolean::toString(core::Float64::near(one, one + 0.1)));
    core::Console::log(temper_globals::console.value, "one.near(one + 0.25, absTol = 0.25): " + core::Boolean::toString(core::Float64::near(one, one + 0.25, nullptr, 0.25)));
    core::Console::log(temper_globals::console.value, "one.near(one + 0.1 + 1e-9, absTol = 0.1): " + core::Boolean::toString(core::Float64::near(one, one + 0.1 + 1e-9, nullptr, 0.1)));
    core::Console::log(temper_globals::console.value, "one.near(one - 0.1, absTol = 0.11): " + core::Boolean::toString(core::Float64::near(one, one - 0.1, nullptr, 0.11)));
    core::Console::log(temper_globals::console.value, "ten.near(ten + 0.1, absTol = 0.011): " + core::Boolean::toString(core::Float64::near(ten, ten + 0.1, nullptr, 0.011)));
    core::Console::log(temper_globals::console.value, "ten.near(ten + 0.1, relTol = 0.011): " + core::Boolean::toString(core::Float64::near(ten, ten + 0.1, 0.011)));
  });
}

}  // namespace ops
