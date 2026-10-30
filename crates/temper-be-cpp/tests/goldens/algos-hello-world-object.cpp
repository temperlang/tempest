// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace hello_world_object {

struct C;

struct C : public std::enable_shared_from_this<C> {
  std::string x;
  std::string y;
  static std::shared_ptr<C> make(std::string x, std::string y);
  void echo(std::string input) const;
  std::string get_x() const;
  std::string get_y() const;
};

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace hello_world_object

// init.cpp
#include <hello-world-object/init.hpp>

#include <mutex>

namespace hello_world_object {

namespace core = temper::core;

static std::shared_ptr<core::Console::Type> console;

std::shared_ptr<C> C::make(std::string x, std::string y) {
  std::shared_ptr<C> self = std::make_shared<C>();
  self->x = x;
  self->y = y;
  return self;
}

void C::echo(std::string input) const {
  core::Console::log(console, input);
}

std::string C::get_x() const {
  return this->x;
}

std::string C::get_y() const {
  return this->y;
}

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    console = core::Console::get_console();
    std::shared_ptr<C> c = C::make("Hello", "World");
    core::Console::log(console, c->x + ", " + c->y + "!");
    c->echo("Hello World, again!");
  });
}

}  // namespace hello_world_object
