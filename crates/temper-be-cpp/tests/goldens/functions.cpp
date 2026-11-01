// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace functions_program {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

struct Counter;
struct IntOp;
struct Scaler;
struct Button;

struct Counter : public virtual temper::core::AnyValueBase {
  virtual int32_t apply() const = 0;
};

struct IntOp : public virtual temper::core::AnyValueBase {
  virtual int32_t apply(int32_t x) const = 0;
};

struct Scaler : public std::enable_shared_from_this<Scaler> {
  static std::shared_ptr<Scaler> make();
  static int32_t triple(int32_t x);
};

struct Button : public std::enable_shared_from_this<Button> {
  std::string label;
  std::function<int32_t()> onPress;
  static std::shared_ptr<Button> make(std::string label, std::function<int32_t()> const & onPress);
  std::string press() const;
  std::string get_label() const;
  std::function<int32_t()> get_onPress() const;
};

}  // namespace functions_program

// init.cpp
#include <functions-program/init.hpp>

namespace functions_program {

namespace core = temper::core;

static std::function<int32_t()> makeCounter(int32_t start) {
  std::shared_ptr<int32_t> count = std::make_shared<int32_t>(start);
  auto fn = [=]() -> int32_t {
    *count = core::Int::add((*count), 1);
    return (*count);
  };
  return fn;
}

static std::function<int32_t(int32_t)> twice(std::function<int32_t(int32_t)> const & f) {
  return [=](int32_t x) -> int32_t { return f(f(x)); };
}

static int32_t applyTo(std::function<int32_t(int32_t)> const & f, int32_t x) {
  return f(x);
}

static int32_t addOne(int32_t x) {
  return core::Int::add(x, 1);
}

std::shared_ptr<Scaler> Scaler::make() {
  std::shared_ptr<Scaler> self = std::make_shared<Scaler>();
  return self;
}

int32_t Scaler::triple(int32_t x) {
  return core::Int::mul(x, 3);
}

std::shared_ptr<Button> Button::make(std::string label, std::function<int32_t()> const & onPress) {
  std::shared_ptr<Button> self = std::make_shared<Button>();
  self->label = label;
  self->onPress = onPress;
  return self;
}

std::string Button::press() const {
  return this->label + " " + std::to_string(this->onPress());
}

std::string Button::get_label() const {
  return this->label;
}

std::function<int32_t()> Button::get_onPress() const {
  return this->onPress;
}

void global_init_init() {
  core::init_once([] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    std::function<int32_t()> counter = makeCounter(10);
    counter();
    core::Console::log(console, "count " + std::to_string(counter()));
    std::function<int32_t()> other = makeCounter(0);
    int32_t int32 = other();
    core::Console::log(console, "other " + std::to_string(int32) + " count " + std::to_string(counter()));
    std::function<int32_t(int32_t)> addTwo = twice(addOne);
    int32_t value = addTwo(5);
    core::Console::log(console, "twice " + std::to_string(value) + " " + std::to_string(applyTo(addOne, 1)));
    int32_t value2 = applyTo(Scaler::triple, 4);
    core::Console::log(console, "triple " + std::to_string(value2) + " " + std::to_string(twice(Scaler::triple)(2)));
    std::shared_ptr<std::vector<int32_t>> ns = core::List::make<int32_t>(5, 2, 8, 1, 4);
    std::shared_ptr<std::vector<int32_t>> doubled = core::List::map(ns, [](int32_t n) -> int32_t { return core::Int::mul(n, 2); });
    std::shared_ptr<std::vector<int32_t>> big = core::List::filter(doubled, [](int32_t n) -> bool { return n > 4; });
    core::Console::log(console, core::List::join(big, ", ", [](int32_t n) -> std::string { return std::to_string(n); }));
    std::shared_ptr<std::vector<int32_t>> order = core::List::sorted(ns, [](int32_t a, int32_t b) -> int32_t { return core::Int::sub(a, b); });
    core::Console::log(console, core::List::join(order, " ", [](int32_t n) -> std::string { return std::to_string(n); }));
    int32_t total = 0;
    {
      int32_t length = core::List::length<int32_t>(ns);
      int32_t i2 = 0;
      while (i2 < length) {
        int32_t n = core::List::get<int32_t>(ns, i2);
        i2 = core::Int::add(i2, 1);
        total = core::Int::add(total, n);
      }
    }
    core::Console::log(console, "total " + std::to_string(total));
    std::shared_ptr<Button> button = Button::make("hits", makeCounter(0));
    button->press();
    core::Console::log(console, button->press());
    std::shared_ptr<std::vector<std::function<int32_t()>>> makers = core::ListBuilder::make<std::function<int32_t()>>();
    {
      int32_t i = 0;
      while (i < 3) {
        int32_t k = core::Int::mul(i, 10);
        core::ListBuilder::add<std::function<int32_t()>>(makers, [=]() -> int32_t { return k; });
        i = core::Int::add(i, 1);
      }
    }
    core::Console::log(console, core::List::join(makers, " ", [](std::function<int32_t()> c) -> std::string { return std::to_string(c()); }));
  });
}

}  // namespace functions_program
