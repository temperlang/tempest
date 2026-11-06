// helpers.hpp
#pragma once

#include <temper-core/core.hpp>

namespace defaults {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_helpers();

struct Pad;

struct Pad : public std::enable_shared_from_this<Pad> {
  int32_t left;
  int32_t right;
  static std::shared_ptr<Pad> make(temper::core::NullableParam<int32_t> left, temper::core::NullableParam<int32_t> right);
  static std::shared_ptr<Pad> make(temper::core::NullableParam<int32_t> left);
  static std::shared_ptr<Pad> make();
  std::string apply(std::string text, temper::core::NullableParam<std::string> fill) const;
  std::string apply(std::string text) const;
  int32_t get_left() const;
  int32_t get_right() const;
};

std::string wrap(std::string text, temper::core::NullableParam<std::string> open, temper::core::NullableParam<std::string> close);
std::string wrap(std::string text, temper::core::NullableParam<std::string> open);
std::string wrap(std::string text);

}  // namespace defaults

// helpers.cpp
#include <defaults/helpers.hpp>

namespace defaults {

namespace core = temper::core;

std::string wrap(std::string text, core::NullableParam<std::string> open, core::NullableParam<std::string> close) {
  std::string open2 = open == nullptr ? "[" : open.value;
  std::string close2 = close == nullptr ? open2 : close.value;
  return open2 + text + close2;
}

std::string wrap(std::string text, core::NullableParam<std::string> open) {
  return wrap(text, open, nullptr);
}

std::string wrap(std::string text) {
  return wrap(text, nullptr, nullptr);
}

std::shared_ptr<Pad> Pad::make(core::NullableParam<int32_t> left, core::NullableParam<int32_t> right) {
  std::shared_ptr<Pad> self = std::make_shared<Pad>();
  int32_t left2 = left == nullptr ? 1 : left.value;
  int32_t right2 = right == nullptr ? 2 : right.value;
  self->left = left2;
  self->right = right2;
  return self;
}

std::shared_ptr<Pad> Pad::make(core::NullableParam<int32_t> left) {
  return make(left, nullptr);
}

std::shared_ptr<Pad> Pad::make() {
  return make(nullptr, nullptr);
}

std::string Pad::apply(std::string text, core::NullableParam<std::string> fill) const {
  std::string fill2 = fill == nullptr ? "." : fill.value;
  std::string out = text;
  {
    int32_t i = 0;
    while (i < this->left) {
      out = fill2 + out;
      i = core::Int::add(i, 1);
    }
  }
  {
    int32_t i = 0;
    while (i < this->right) {
      out = out + fill2;
      i = core::Int::add(i, 1);
    }
  }
  return out;
}

std::string Pad::apply(std::string text) const {
  return apply(text, nullptr);
}

int32_t Pad::get_left() const {
  return this->left;
}

int32_t Pad::get_right() const {
  return this->right;
}

void global_init_helpers() {}

}  // namespace defaults

// init.hpp
#pragma once

#include <temper-core/core.hpp>
#include <defaults/helpers.hpp>

namespace defaults {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

struct Greeter;
struct Counter;

struct Greeter : public std::enable_shared_from_this<Greeter> {
  std::string greeting;
  std::string punct;
  static std::shared_ptr<Greeter> make(temper::core::NullableParam<std::string> greeting, temper::core::NullableParam<std::string> punct);
  static std::shared_ptr<Greeter> make(temper::core::NullableParam<std::string> greeting);
  static std::shared_ptr<Greeter> make();
  std::string greet(temper::core::NullableParam<std::string> name, temper::core::NullableParam<int32_t> times) const;
  std::string greet(temper::core::NullableParam<std::string> name) const;
  std::string greet() const;
  static std::string shout(std::string text, temper::core::NullableParam<std::string> mark);
  static std::string shout(std::string text);
  std::string get_greeting() const;
  std::string get_punct() const;
};

struct Counter : public std::enable_shared_from_this<Counter> {
  int32_t count;
  static std::shared_ptr<Counter> make(temper::core::NullableParam<int32_t> start);
  static std::shared_ptr<Counter> make();
  int32_t add(temper::core::NullableParam<int32_t> by);
  int32_t add();
  int32_t get_count();
};

}  // namespace defaults

// init.cpp
#include <defaults/init.hpp>

#include <mutex>

namespace defaults {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<int32_t> calls;

}  // namespace temper_globals

static std::mutex moduleMutex;

static int32_t next() {
  std::lock_guard<std::mutex> lock(moduleMutex);
  temper_globals::calls.value = core::Int::add(temper_globals::calls.value, 1);
  return temper_globals::calls.value;
}

static std::string range(int32_t start, core::NullableParam<int32_t> end, core::NullableParam<int32_t> step) {
  int32_t end2 = end == nullptr ? core::Int::add(start, 10) : end.value;
  int32_t step2 = step == nullptr ? next() : step.value;
  return std::to_string(start) + ".." + std::to_string(end2) + " by " + std::to_string(step2);
}

static std::string range(int32_t start, core::NullableParam<int32_t> end) {
  return range(start, end, nullptr);
}

static std::string range(int32_t start) {
  return range(start, nullptr, nullptr);
}

static std::string tag(std::string text, core::NullableParam<std::string> mark) {
  if (mark == nullptr) {
    return text;
  } else {
    return mark.value + text;
  }
}

static std::string tag(std::string text) {
  return tag(text, nullptr);
}

static std::string named(core::NullableParam<std::string> name) {
  core::NullableParam<std::string> name2 = name == nullptr ? core::NullableParam<std::string>("nobody") : name;
  {
    if (name2 == nullptr) {
      return "null";
    } else {
      return name2.value;
    }
  }
}

static std::string named() {
  return named(nullptr);
}

std::shared_ptr<Greeter> Greeter::make(core::NullableParam<std::string> greeting, core::NullableParam<std::string> punct) {
  std::shared_ptr<Greeter> self = std::make_shared<Greeter>();
  std::string greeting2 = greeting == nullptr ? "Hello" : greeting.value;
  std::string punct2 = punct == nullptr ? "!" : punct.value;
  self->greeting = greeting2;
  self->punct = punct2;
  return self;
}

std::shared_ptr<Greeter> Greeter::make(core::NullableParam<std::string> greeting) {
  return make(greeting, nullptr);
}

std::shared_ptr<Greeter> Greeter::make() {
  return make(nullptr, nullptr);
}

std::string Greeter::greet(core::NullableParam<std::string> name, core::NullableParam<int32_t> times) const {
  std::string name2 = name == nullptr ? "world" : name.value;
  int32_t times2 = times == nullptr ? 1 : times.value;
  std::string out = "";
  {
    int32_t i = 0;
    while (i < times2) {
      out = out + this->greeting + ", " + name2 + this->punct;
      i = core::Int::add(i, 1);
    }
  }
  return out;
}

std::string Greeter::greet(core::NullableParam<std::string> name) const {
  return greet(name, nullptr);
}

std::string Greeter::greet() const {
  return greet(nullptr, nullptr);
}

std::string Greeter::shout(std::string text, core::NullableParam<std::string> mark) {
  std::string mark2 = mark == nullptr ? "!" : mark.value;
  return text + mark2 + mark2;
}

std::string Greeter::shout(std::string text) {
  return shout(text, nullptr);
}

std::string Greeter::get_greeting() const {
  return this->greeting;
}

std::string Greeter::get_punct() const {
  return this->punct;
}

std::shared_ptr<Counter> Counter::make(core::NullableParam<int32_t> start) {
  std::shared_ptr<Counter> self = std::make_shared<Counter>();
  int32_t start2 = start == nullptr ? 10 : start.value;
  self->count = start2;
  return self;
}

std::shared_ptr<Counter> Counter::make() {
  return make(nullptr);
}

int32_t Counter::add(core::NullableParam<int32_t> by) {
  int32_t by2 = by == nullptr ? 1 : by.value;
  this->count = core::Int::add(this->count, by2);
  return this->count;
}

int32_t Counter::add() {
  return add(nullptr);
}

int32_t Counter::get_count() {
  return this->count;
}

void global_init_init() {
  core::init_once([] {
    temper_globals::calls.construct();
    global_init_helpers();
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    temper_globals::calls.value = 0;
    core::Console::log(console, defaults::range(1));
    core::Console::log(console, defaults::range(1, 5));
    core::Console::log(console, defaults::range(1, 5, 3));
    core::Console::log(console, defaults::range(2, nullptr, 7));
    core::Console::log(console, "calls " + std::to_string(temper_globals::calls.value));
    core::Console::log(console, defaults::tag("plain"));
    core::Console::log(console, defaults::tag("marked", "#"));
    core::Console::log(console, defaults::named());
    core::Console::log(console, defaults::named(nullptr));
    core::Console::log(console, defaults::named("Ada"));
    core::Console::log(console, Greeter::make()->greet());
    core::Console::log(console, Greeter::make("Hi")->greet("you", 2));
    core::Console::log(console, Greeter::make(nullptr, "?")->greet(nullptr, 2));
    core::Console::log(console, Greeter::shout("hey"));
    std::shared_ptr<Counter> counter = Counter::make();
    counter->add();
    int32_t int32 = counter->add(5);
    core::Console::log(console, "count " + std::to_string(int32) + " " + std::to_string(Counter::make(1)->add()));
    core::Console::log(console, defaults::wrap("a"));
    core::Console::log(console, defaults::wrap("b", "<"));
    core::Console::log(console, defaults::wrap("c", "<", ">"));
    std::shared_ptr<Pad> pad = Pad::make();
    core::Console::log(console, pad->apply("x"));
    core::Console::log(console, Pad::make(0, 3)->apply("y", "-"));
  });
}

}  // namespace defaults

// main.cpp
#include "defaults/helpers.hpp"
#include "defaults/init.hpp"

int main() {
  defaults::global_init_helpers();
  defaults::global_init_init();
}

// CMakeLists.txt
# Generated by Temper.
cmake_minimum_required(VERSION 3.10)
project(defaults LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
find_package(Threads REQUIRED)

add_library(defaults helpers.cpp init.cpp)
target_include_directories(defaults PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/..)
target_link_libraries(defaults PUBLIC Threads::Threads)

add_executable(defaults-main main.cpp)
target_link_libraries(defaults-main PRIVATE defaults)

