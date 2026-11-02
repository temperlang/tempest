// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace deque {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

}  // namespace deque

// init.cpp
#include <deque/init.hpp>

namespace deque {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;
static core::GlobalSlot<std::shared_ptr<std::deque<std::string>>> deque;

}  // namespace temper_globals

static void checkEmpty() {
  core::Console::log(temper_globals::console.value, core::Deque::isEmpty<std::string>(temper_globals::deque.value) ? "Empty" : "Not empty");
}

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::deque.construct();
    temper_globals::console.value = core::Console::get_console();
    temper_globals::deque.value = core::Deque::make<std::string>();
    checkEmpty();
    if (!core::Deque::isEmpty<std::string>(temper_globals::deque.value)) {
      core::Console::log(temper_globals::console.value, core::Deque::removeFirst<std::string>(temper_globals::deque.value));
    }
    core::Deque::add<std::string>(temper_globals::deque.value, "foo");
    core::Deque::add<std::string>(temper_globals::deque.value, "bar");
    core::Console::log(temper_globals::console.value, core::Deque::removeFirst<std::string>(temper_globals::deque.value));
    core::Deque::add<std::string>(temper_globals::deque.value, "baz");
    core::Console::log(temper_globals::console.value, core::Deque::removeFirst<std::string>(temper_globals::deque.value));
    core::Deque::add<std::string>(temper_globals::deque.value, "boo");
    checkEmpty();
    while (!core::Deque::isEmpty<std::string>(temper_globals::deque.value)) {
      core::Console::log(temper_globals::console.value, core::Deque::removeFirst<std::string>(temper_globals::deque.value));
    }
    checkEmpty();
  });
}

}  // namespace deque
