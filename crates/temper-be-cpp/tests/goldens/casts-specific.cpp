// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace specific {

struct AlphaInterface;
struct BravoClass;
struct CharlieClass;

struct AlphaInterface : public virtual temper::core::AnyValueBase {};

struct BravoClass : public virtual AlphaInterface {
  static std::shared_ptr<BravoClass> make();
};

struct CharlieClass : public virtual AlphaInterface {
  static std::shared_ptr<CharlieClass> make();
};

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace specific

// init.cpp
#include <specific/init.hpp>

#include <mutex>

namespace specific {

namespace core = temper::core;

static std::shared_ptr<core::Console::Type> console;

std::shared_ptr<BravoClass> BravoClass::make() {
  std::shared_ptr<BravoClass> self = std::make_shared<BravoClass>();
  return self;
}

std::shared_ptr<CharlieClass> CharlieClass::make() {
  std::shared_ptr<CharlieClass> self = std::make_shared<CharlieClass>();
  return self;
}

static void checkB(std::shared_ptr<AlphaInterface> const & instance) {
  std::string value;
  try {
    core::checked_cast<BravoClass>(instance);
    value = "yep";
  } catch (core::TemperBubble const &) {
    value = "nope";
  }
  core::Console::log(console, value);
}

static void checkIfB(std::shared_ptr<AlphaInterface> const & instance) {
  core::Console::log(console, dynamic_cast<BravoClass const *>(instance.get()) != nullptr ? "yep" : "nope");
}

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    console = core::Console::get_console();
    checkB(BravoClass::make());
    checkB(CharlieClass::make());
    std::shared_ptr<std::vector<std::shared_ptr<AlphaInterface>>> items = core::List::make<std::shared_ptr<AlphaInterface>>(BravoClass::make(), CharlieClass::make());
    checkB(core::List::get<std::shared_ptr<AlphaInterface>>(items, 0));
    checkB(core::List::get<std::shared_ptr<AlphaInterface>>(items, 1));
    checkIfB(core::List::get<std::shared_ptr<AlphaInterface>>(items, 0));
    checkIfB(core::List::get<std::shared_ptr<AlphaInterface>>(items, 1));
  });
}

}  // namespace specific
