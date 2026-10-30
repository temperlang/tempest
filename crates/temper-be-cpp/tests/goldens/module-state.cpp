// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace module_state {

extern std::string greeting;
// The module's functions write it under their lock, so read it only
// while no other thread may be calling them.
extern int32_t total;
// The module's functions write it under their lock, so read it only
// while no other thread may be calling them.
extern int32_t ticks;

void tick();
std::string report();

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace module_state

// init.cpp
#include <module-state/init.hpp>

#include <mutex>

namespace module_state {

namespace core = temper::core;

std::string greeting;
static int32_t count;
static int32_t limit;
int32_t total;
int32_t ticks;
static std::mutex moduleMutex;

// Reads a variable that functions write, under the module's lock.
template <class T>
static T load(T const &variable) {
  std::lock_guard<std::mutex> lock(moduleMutex);
  return variable;
}

static int32_t bump() {
  std::lock_guard<std::mutex> lock(moduleMutex);
  count = core::Int::add(count, 1);
  total = core::Int::add(total, count);
  return count;
}

void tick() {
  std::lock_guard<std::mutex> lock(moduleMutex);
  ticks = core::Int::add(ticks, 1);
}

std::string report() {
  int32_t int32 = bump();
  return greeting + " " + std::to_string(int32) + " of " + std::to_string(limit) + " " + std::to_string(load(total));
}

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    greeting = "hi";
    count = 0;
    limit = 3;
    total = 10;
    ticks = 0;
    int32_t local = 1;
    local = core::Int::add(local, limit);
    core::Console::log(console, report());
    core::Console::log(console, report());
    core::Console::log(console, "local " + std::to_string(local));
  });
}

}  // namespace module_state
