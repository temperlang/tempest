// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace hello_from_class_to_top {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

struct Something;

struct Something : public std::enable_shared_from_this<Something> {
  std::string message;
  static std::shared_ptr<Something> make(std::string message);
  void sayMessage() const;
};

}  // namespace hello_from_class_to_top

// init.cpp
#include <hello-from-class-to-top/init.hpp>

namespace hello_from_class_to_top {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;

}  // namespace temper_globals

static void say(std::string message) {
  core::Console::log(temper_globals::console.value, message);
}

std::shared_ptr<Something> Something::make(std::string message) {
  std::shared_ptr<Something> self = std::make_shared<Something>();
  self->message = message;
  return self;
}

void Something::sayMessage() const {
  std::shared_ptr<Something const> self = shared_from_this();
  hello_from_class_to_top::say(self->message);
}

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::console.value = core::Console::get_console();
    std::shared_ptr<Something> something = Something::make("Hello, World!");
    something->sayMessage();
  });
}

}  // namespace hello_from_class_to_top
