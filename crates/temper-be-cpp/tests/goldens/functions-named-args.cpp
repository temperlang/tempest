// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace named_args {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

struct Greet;
struct Person;
struct CalcOptions;

struct Greet : public std::enable_shared_from_this<Greet> {
  static std::shared_ptr<Greet> make(temper::core::NullableParam<std::string> message, temper::core::NullableParam<bool> repeat);
  static std::shared_ptr<Greet> make(temper::core::NullableParam<std::string> message);
  static std::shared_ptr<Greet> make();
};

struct Person : public std::enable_shared_from_this<Person> {
  std::string name;
  int32_t age;
  static std::shared_ptr<Person> make(temper::core::NullableParam<std::string> name, temper::core::NullableParam<int32_t> age);
  static std::shared_ptr<Person> make(temper::core::NullableParam<std::string> name);
  static std::shared_ptr<Person> make();
  std::string toString() const;
  void greet(temper::core::NullableParam<std::string> greeting) const;
  void greet() const;
  std::string get_name() const;
  int32_t get_age() const;
};

struct CalcOptions : public std::enable_shared_from_this<CalcOptions> {
  int32_t scale;
  int32_t origin;
  static std::shared_ptr<CalcOptions> make(temper::core::NullableParam<int32_t> scale, temper::core::NullableParam<int32_t> origin);
  static std::shared_ptr<CalcOptions> make(temper::core::NullableParam<int32_t> scale);
  static std::shared_ptr<CalcOptions> make();
  int32_t get_scale() const;
  int32_t get_origin() const;
};

}  // namespace named_args

// init.cpp
#include <named-args/init.hpp>

#include <mutex>

namespace named_args {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;
static core::GlobalSlot<std::string> message;
static core::GlobalSlot<bool> repeat;

}  // namespace temper_globals

static std::mutex moduleMutex;

std::shared_ptr<Greet> Greet::make(core::NullableParam<std::string> message, core::NullableParam<bool> repeat) {
  global_init_init();
  std::shared_ptr<Greet> self = std::make_shared<Greet>();
  std::string message2 = message == nullptr ? "Hi!" : message.value;
  bool repeat2 = repeat == nullptr ? false : repeat.value;
  core::Console::log(temper_globals::console.value, message2);
  if (repeat2) {
    core::Console::log(temper_globals::console.value, message2);
  }
  return self;
}

std::shared_ptr<Greet> Greet::make(core::NullableParam<std::string> message) {
  return make(message, nullptr);
}

std::shared_ptr<Greet> Greet::make() {
  return make(nullptr, nullptr);
}

static std::string changeRepeat() {
  std::lock_guard<std::mutex> lock(moduleMutex);
  temper_globals::repeat.value = true;
  return temper_globals::message.value;
}

static bool changeMessage() {
  std::lock_guard<std::mutex> lock(moduleMutex);
  temper_globals::message.value = "Namaste!";
  return temper_globals::repeat.value;
}

std::shared_ptr<Person> Person::make(core::NullableParam<std::string> name, core::NullableParam<int32_t> age) {
  std::shared_ptr<Person> self = std::make_shared<Person>();
  std::string name2 = name == nullptr ? "Alice" : name.value;
  int32_t age2 = age == nullptr ? 40 : age.value;
  self->name = name2;
  self->age = age2;
  return self;
}

std::shared_ptr<Person> Person::make(core::NullableParam<std::string> name) {
  return make(name, nullptr);
}

std::shared_ptr<Person> Person::make() {
  return make(nullptr, nullptr);
}

std::string Person::toString() const {
  return "{ class: Person, name: " + this->name + ", age: " + std::to_string(this->age) + " }";
}

void Person::greet(core::NullableParam<std::string> greeting) const {
  std::string greeting2 = greeting == nullptr ? "Hi" : greeting.value;
  core::Console::log(temper_globals::console.value, greeting2 + ", " + this->name);
}

void Person::greet() const {
  return greet(nullptr);
}

std::string Person::get_name() const {
  return this->name;
}

int32_t Person::get_age() const {
  return this->age;
}

std::shared_ptr<CalcOptions> CalcOptions::make(core::NullableParam<int32_t> scale, core::NullableParam<int32_t> origin) {
  std::shared_ptr<CalcOptions> self = std::make_shared<CalcOptions>();
  int32_t scale2 = scale == nullptr ? 1 : scale.value;
  int32_t origin2 = origin == nullptr ? 0 : origin.value;
  self->scale = scale2;
  self->origin = origin2;
  return self;
}

std::shared_ptr<CalcOptions> CalcOptions::make(core::NullableParam<int32_t> scale) {
  return make(scale, nullptr);
}

std::shared_ptr<CalcOptions> CalcOptions::make() {
  return make(nullptr, nullptr);
}

int32_t CalcOptions::get_scale() const {
  return this->scale;
}

int32_t CalcOptions::get_origin() const {
  return this->origin;
}

template <class T>
static T calc(int32_t n, std::function<T(int32_t)> const & transform, std::shared_ptr<CalcOptions> const & options) {
  std::shared_ptr<CalcOptions> options2 = options == nullptr ? CalcOptions::make() : options;
  int32_t origin = options2->origin;
  int32_t scale = options2->scale;
  return transform(core::Int::mul(core::Int::sub(n, origin), scale));
}

template <class T>
static T calc(int32_t n, std::function<T(int32_t)> const & transform) {
  return calc(n, transform, nullptr);
}

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::message.construct();
    temper_globals::repeat.construct();
    temper_globals::console.value = core::Console::get_console();
    Greet::make();
    Greet::make("Bye!");
    Greet::make("Hola!", true);
    Greet::make("Salut!");
    Greet::make(nullptr, true);
    Greet::make(nullptr, true);
    {
      bool arg = true;
      Greet::make("Ni hao!", arg);
    }
    temper_globals::message.value = "Hi!";
    temper_globals::repeat.value = false;
    {
      bool arg2 = changeMessage();
      std::string arg3 = changeRepeat();
      Greet::make(arg3, arg2);
    }
    core::Console::log(temper_globals::console.value, Person::make()->toString());
    core::Console::log(temper_globals::console.value, Person::make("Bob")->toString());
    core::Console::log(temper_globals::console.value, Person::make("Carrie", 50)->toString());
    Person::make("Dan")->greet("Hallo");
    core::Console::log(temper_globals::console.value, named_args::calc<std::string>(2, [](int32_t n) -> std::string { return std::to_string(n); }));
    core::Console::log(temper_globals::console.value, named_args::calc<std::string>(2, [](int32_t n) -> std::string { return std::to_string(n); }, CalcOptions::make(3, 4)));
    core::Console::log(temper_globals::console.value, named_args::calc<std::string>(2, [](int32_t n) -> std::string { return std::to_string(n); }, CalcOptions::make(3)));
    core::Console::log(temper_globals::console.value, named_args::calc<std::string>(2, [](int32_t n) -> std::string { return std::to_string(n); }, CalcOptions::make(nullptr, 4)));
    auto fn5 = [](int32_t n) -> std::shared_ptr<core::Empty> {
      core::Console::log(temper_globals::console.value, std::to_string(n));
      return core::empty();
    };
    named_args::calc<std::shared_ptr<core::Empty>>(2, fn5, CalcOptions::make(-2));
  });
}

}  // namespace named_args
