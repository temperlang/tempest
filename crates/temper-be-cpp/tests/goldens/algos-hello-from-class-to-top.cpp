// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace hello_from_class_to_top {

struct Something;

struct Something : public std::enable_shared_from_this<Something> {
  std::string message;
  static std::shared_ptr<Something> make(std::string message);
  void sayMessage() const;
};

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace hello_from_class_to_top

// init.cpp
#include <hello-from-class-to-top/init.hpp>

#include <mutex>

namespace hello_from_class_to_top {

namespace core = temper::core;

static std::shared_ptr<core::Console::Type> console;

static void say(std::string message) {
  core::Console::log(console, message);
}

std::shared_ptr<Something> Something::make(std::string message) {
  std::shared_ptr<Something> self = std::make_shared<Something>();
  self->message = message;
  return self;
}

void Something::sayMessage() const {
  std::shared_ptr<Something const> self = shared_from_this();
  say(self->message);
}

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    console = core::Console::get_console();
    std::shared_ptr<Something> something = Something::make("Hello, World!");
    something->sayMessage();
  });
}

}  // namespace hello_from_class_to_top
