// init.hpp
#pragma once

#include <temper-core/core.hpp>
#include <typeindex>

namespace angle_call {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

template <class T>
struct Blah;
struct Hi;
struct Classy;
struct Picker;

template <class T>
struct Blah : public std::enable_shared_from_this<Blah<T>> {
  static std::shared_ptr<Blah<T>> make();
};

struct Hi : public std::enable_shared_from_this<Hi> {
  std::shared_ptr<Blah<std::string>> thing;
  static std::shared_ptr<Hi> make(std::shared_ptr<Blah<std::string>> const & thing);
  std::shared_ptr<Blah<std::string>> get_thing() const;
};

struct Classy : public std::enable_shared_from_this<Classy> {
  int32_t index;
  static std::shared_ptr<Classy> make(int32_t index);
  template <class T>
  static T first(std::shared_ptr<std::vector<T>> const & things);
  template <class T>
  T indexed(std::shared_ptr<std::vector<T>> const & things) const;
  int32_t get_index() const;
};

struct Picker : public std::enable_shared_from_this<Picker> {
  static std::shared_ptr<Picker> make();
  template <class T>
  T pick(std::shared_ptr<std::vector<T>> const & things) const;
};

template <class T>
std::shared_ptr<Blah<T>> Blah<T>::make() {
  std::shared_ptr<Blah<T>> self = std::make_shared<Blah<T>>();
  return self;
}

}  // namespace angle_call

// init.cpp
#include <angle-call/init.hpp>

namespace angle_call {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;

}  // namespace temper_globals

std::shared_ptr<Hi> Hi::make(std::shared_ptr<Blah<std::string>> const & thing) {
  std::shared_ptr<Hi> self = std::make_shared<Hi>();
  self->thing = thing;
  return self;
}

std::shared_ptr<Blah<std::string>> Hi::get_thing() const {
  return this->thing;
}

static std::type_index whatever(int32_t n) {
  if (n == 0) {
    core::Console::log(temper_globals::console.value, "0");
  }
  return std::type_index(typeid(void));
}

std::shared_ptr<Classy> Classy::make(int32_t index) {
  std::shared_ptr<Classy> self = std::make_shared<Classy>();
  self->index = index;
  return self;
}

template <class T>
T Classy::first(std::shared_ptr<std::vector<T>> const & things) {
  T thing = core::List::get<T>(things, 0);
  return thing;
}

template <class T>
T Classy::indexed(std::shared_ptr<std::vector<T>> const & things) const {
  T thing = core::List::get<T>(things, this->index);
  return thing;
}

int32_t Classy::get_index() const {
  return this->index;
}

template <class T>
static T lastOf(std::shared_ptr<std::vector<T>> const & things) {
  T thing = core::List::get<T>(things, core::Int::sub(core::List::length<T>(things), 1));
  return thing;
}

std::shared_ptr<Picker> Picker::make() {
  std::shared_ptr<Picker> self = std::make_shared<Picker>();
  return self;
}

template <class T>
T Picker::pick(std::shared_ptr<std::vector<T>> const & things) const {
  return angle_call::lastOf<T>(things);
}

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::console.value = core::Console::get_console();
    whatever(1);
    std::shared_ptr<std::vector<std::string>> someThings = core::List::make<std::string>("a", "b");
    std::string first = Classy::first<std::string>(someThings);
    std::string second = Classy::make(1)->indexed<std::string>(someThings);
    core::Console::log(temper_globals::console.value, "Some things: " + first + ", " + second);
    core::Console::log(temper_globals::console.value, "Picked: " + Picker::make()->pick<std::string>(someThings));
  });
}

}  // namespace angle_call
