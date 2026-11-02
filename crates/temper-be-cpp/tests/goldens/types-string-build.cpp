// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace build {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

struct StringBuilderOwner;

struct StringBuilderOwner : public std::enable_shared_from_this<StringBuilderOwner> {
  std::shared_ptr<std::ostringstream> builder;
  static std::shared_ptr<StringBuilderOwner> make(std::shared_ptr<std::ostringstream> const & builder);
  std::shared_ptr<std::ostringstream> get_builder() const;
};

}  // namespace build

// init.cpp
#include <build/init.hpp>

namespace build {

namespace core = temper::core;

std::shared_ptr<StringBuilderOwner> StringBuilderOwner::make(std::shared_ptr<std::ostringstream> const & builder) {
  std::shared_ptr<StringBuilderOwner> self = std::make_shared<StringBuilderOwner>();
  self->builder = builder;
  return self;
}

std::shared_ptr<std::ostringstream> StringBuilderOwner::get_builder() const {
  return this->builder;
}

void global_init_init() {
  core::init_once([] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    std::shared_ptr<std::ostringstream> builder = core::StringBuilder::make();
    core::StringBuilder::append(builder, "Hi ");
    core::StringBuilder::appendCodepoint(builder, 127759);
    std::string source = "Wow!! Neato!";
    int32_t bangIndex = core::String::step(source, core::String::begin(), 3);
    int32_t spaceIndex = core::String::step(source, bangIndex, 2);
    core::StringBuilder::appendBetween(builder, source, bangIndex, spaceIndex);
    std::string built = core::StringBuilder::toString(builder);
    core::StringBuilder::append(builder, "\n");
    core::StringBuilder::append(builder, built);
    core::Console::log(console, core::StringBuilder::toString(builder));
    try {
      core::StringBuilder::appendCodepoint(builder, 1114112);
    } catch (core::TemperBubble const &) {
      core::Console::log(console, "High code blocked.");
    }
    try {
      core::StringBuilder::appendCodepoint(builder, 55296);
    } catch (core::TemperBubble const &) {
      core::Console::log(console, "Surrogate blocked.");
    }
    core::Console::log(console, core::StringBuilder::toString(builder));
    core::StringBuilder::clear(builder);
    core::StringBuilder::append(builder, "Back to square zero.");
    core::Console::log(console, core::StringBuilder::toString(builder));
    std::shared_ptr<StringBuilderOwner> owner = StringBuilderOwner::make(core::StringBuilder::make());
    core::StringBuilder::append(owner->builder, "Hi from inside.");
    core::Console::log(console, core::StringBuilder::toString(owner->builder));
  });
}

}  // namespace build
