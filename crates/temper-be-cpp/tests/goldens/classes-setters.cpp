// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace setters {

struct Foo;
struct Bar;
struct Whatever;
struct ConcreteWhatever;

struct Foo : public std::enable_shared_from_this<Foo> {
  std::string backing;
  static std::shared_ptr<Foo> make(std::string backing);
  std::string get_bar();
  void set_bar(std::string value);
};

struct Bar : public std::enable_shared_from_this<Bar> {
  std::string backing;
  static std::shared_ptr<Bar> make(std::string backing);
  std::string get_bar();
  void set_bar(std::string value);
  void usingSet(std::string value);
};

struct Whatever : public virtual temper::core::AnyValueBase {
  virtual int32_t get_something() = 0;
  virtual void set_something(int32_t blah) = 0;
};

struct ConcreteWhatever : public virtual Whatever {
  int32_t something;
  static std::shared_ptr<ConcreteWhatever> make(int32_t something);
  int32_t get_something() override;
  void set_something(int32_t value) override;
};

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace setters

// init.cpp
#include <setters/init.hpp>

#include <mutex>

namespace setters {

namespace core = temper::core;

std::shared_ptr<Foo> Foo::make(std::string backing) {
  std::shared_ptr<Foo> self = std::make_shared<Foo>();
  self->backing = backing;
  return self;
}

std::string Foo::get_bar() {
  return this->backing;
}

void Foo::set_bar(std::string value) {
  this->backing = value;
}

std::shared_ptr<Bar> Bar::make(std::string backing) {
  std::shared_ptr<Bar> self = std::make_shared<Bar>();
  self->backing = backing;
  return self;
}

std::string Bar::get_bar() {
  return this->backing;
}

void Bar::set_bar(std::string value) {
  this->backing = value;
}

void Bar::usingSet(std::string value) {
  std::shared_ptr<Bar> self = shared_from_this();
  self->set_bar(value);
}

std::shared_ptr<ConcreteWhatever> ConcreteWhatever::make(int32_t something) {
  std::shared_ptr<ConcreteWhatever> self = std::make_shared<ConcreteWhatever>();
  self->something = something;
  return self;
}

int32_t ConcreteWhatever::get_something() {
  return this->something;
}

void ConcreteWhatever::set_something(int32_t value) {
  this->something = value;
}

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    core::Console::log(console, Foo::make("initial")->get_bar());
    std::shared_ptr<Foo> foo = Foo::make("initial");
    foo->set_bar("next");
    core::Console::log(console, foo->get_bar());
    std::shared_ptr<Bar> bar = Bar::make("initial");
    bar->usingSet("next");
    core::Console::log(console, bar->get_bar());
    std::shared_ptr<Whatever> what = ConcreteWhatever::make(4);
    what->set_something(5);
    core::Console::log(console, "something set to " + std::to_string(what->get_something()));
  });
}

}  // namespace setters
