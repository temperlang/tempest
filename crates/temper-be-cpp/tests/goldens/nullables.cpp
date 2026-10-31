// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace nullables {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

struct Node;

struct Node : public std::enable_shared_from_this<Node> {
  int32_t value;
  std::shared_ptr<Node> next;
  static std::shared_ptr<Node> make(int32_t value, std::shared_ptr<Node> const & next);
  int32_t last() const;
  int32_t get_value() const;
  std::shared_ptr<Node> get_next() const;
};

std::string describe(temper::core::NullableParam<int32_t> x);
temper::core::NullableParam<std::string> firstWord(temper::core::NullableParam<std::string> s);

}  // namespace nullables

// init.cpp
#include <nullables/init.hpp>

namespace nullables {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;

}  // namespace temper_globals

std::shared_ptr<Node> Node::make(int32_t value, std::shared_ptr<Node> const & next) {
  global_init_init();
  std::shared_ptr<Node> self = std::make_shared<Node>();
  self->value = value;
  self->next = next;
  return self;
}

int32_t Node::last() const {
  std::shared_ptr<Node const> self = shared_from_this();
  std::shared_ptr<Node> n = self->next;
  if (n == nullptr) {
    return self->value;
  } else {
    return n->last();
  }
}

int32_t Node::get_value() const {
  return this->value;
}

std::shared_ptr<Node> Node::get_next() const {
  return this->next;
}

std::string describe(core::NullableParam<int32_t> x) {
  global_init_init();
  if (x == nullptr) {
    return "none";
  }
  return "some " + std::to_string(core::Int::add(x.value, 1));
}

core::NullableParam<std::string> firstWord(core::NullableParam<std::string> s) {
  global_init_init();
  if (s != nullptr && s.value != "") {
    return s;
  } else {
    return nullptr;
  }
}

static bool same(core::NullableParam<std::string> a, core::NullableParam<std::string> b) {
  return a == b;
}

static bool sameInt(core::NullableParam<int32_t> a, core::NullableParam<int32_t> b) {
  return a == b;
}

static int32_t parse(std::string s) {
  try {
    return core::String::toInt32(s);
  } catch (core::TemperBubble const &) {
    return -1;
  }
}

static int32_t positive(int32_t n) {
  if (n < 0) {
    core::bubble();
  }
  return n;
}

static int32_t checked(int32_t n) {
  try {
    return positive(n);
  } catch (core::TemperBubble const &) {
    core::panic();
  }
}

static int32_t orZero(core::NullableParam<int32_t> x) {
  {
    if (x == nullptr) {
      return 0;
    } else {
      return x.value;
    }
  }
}

static void varying() {
  core::NullableParam<int32_t> maybe = nullptr;
  core::Console::log(temper_globals::console.value, core::Boolean::toString(maybe == nullptr));
  maybe = 3;
  core::Console::log(temper_globals::console.value, core::Boolean::toString(maybe != nullptr));
}

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::console.value = core::Console::get_console();
    core::Console::log(temper_globals::console.value, describe(nullptr));
    core::Console::log(temper_globals::console.value, describe(41));
    core::NullableParam<std::string> subject = firstWord("hi");
    core::Console::log(temper_globals::console.value, subject == nullptr ? "nothing" : subject.value);
    core::NullableParam<std::string> subject2 = firstWord("");
    core::Console::log(temper_globals::console.value, subject2 == nullptr ? "nothing" : subject2.value);
    core::Console::log(temper_globals::console.value, core::Boolean::toString(same(nullptr, nullptr)) + " " + core::Boolean::toString(same("a", nullptr)) + " " + core::Boolean::toString(same(nullptr, "a")) + " " + core::Boolean::toString(same("a", "a")));
    core::Console::log(temper_globals::console.value, core::Boolean::toString(sameInt(1, 2)) + " " + core::Boolean::toString(sameInt(3, 3)) + " " + core::Boolean::toString(sameInt(nullptr, 3)));
    int32_t int32 = parse("12");
    core::Console::log(temper_globals::console.value, std::to_string(int32) + " " + std::to_string(parse("x")));
    core::Console::log(temper_globals::console.value, std::to_string(checked(7)));
    core::Console::log(temper_globals::console.value, std::to_string(orZero(nullptr)) + " " + std::to_string(orZero(5)));
    varying();
    std::shared_ptr<Node> list = Node::make(1, Node::make(2, Node::make(3, nullptr)));
    core::Console::log(temper_globals::console.value, std::to_string(list->last()));
    std::shared_ptr<Node> tail = list->next;
    if (tail != nullptr) {
      core::Console::log(temper_globals::console.value, std::to_string(tail->value));
    }
    try {
      positive(-1);
    } catch (core::TemperBubble const &) {
      core::Console::log(temper_globals::console.value, "negative");
    }
    int32_t fallback;
    try {
      fallback = positive(-2);
    } catch (core::TemperBubble const &) {
      fallback = 0;
    }
    core::Console::log(temper_globals::console.value, std::to_string(fallback));
  });
}

}  // namespace nullables
