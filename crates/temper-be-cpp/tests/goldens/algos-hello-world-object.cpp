// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace hello_world_object {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

struct C;

struct C : public std::enable_shared_from_this<C> {
  std::string x;
  std::string y;
  static std::shared_ptr<C> make(std::string x, std::string y);
  void echo(std::string input) const;
  std::string get_x() const;
  std::string get_y() const;
};

}  // namespace hello_world_object

// init.cpp
#include <hello-world-object/init.hpp>

namespace hello_world_object {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;

}  // namespace temper_globals

std::shared_ptr<C> C::make(std::string x, std::string y) {
  std::shared_ptr<C> self = std::make_shared<C>();
  self->x = x;
  self->y = y;
  return self;
}

void C::echo(std::string input) const {
  core::Console::log(temper_globals::console.value, input);
}

std::string C::get_x() const {
  return this->x;
}

std::string C::get_y() const {
  return this->y;
}

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::console.value = core::Console::get_console();
    std::shared_ptr<C> c = C::make("Hello", "World");
    core::Console::log(temper_globals::console.value, c->x + ", " + c->y + "!");
    c->echo("Hello World, again!");
  });
}

}  // namespace hello_world_object
