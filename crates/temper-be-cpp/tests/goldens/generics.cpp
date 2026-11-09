// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace generics {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

template <class T>
struct Holder;
template <class T>
struct Box;
struct Named;
template <class T>
struct Shelf;

template <class T>
struct Holder : public virtual temper::core::AnyValueBase {
  virtual T get_held() const = 0;
};

template <class T>
struct Box : public virtual Holder<T> {
  T value;
  static std::shared_ptr<Box<T>> make(T value);
  T get_held() const override;
  std::shared_ptr<temper::core::Pair<T, int32_t>> pairWith(int32_t other) const;
  T get_value() const;
};

struct Named : public std::enable_shared_from_this<Named> {
  temper::core::Str name;
  static std::shared_ptr<Named> make(temper::core::Str const & name);
  temper::core::Str get_name() const;
};

template <class T>
struct Shelf : public std::enable_shared_from_this<Shelf<T>> {
  std::shared_ptr<std::vector<T>> items;
  static std::shared_ptr<Shelf<T>> make(std::shared_ptr<std::vector<T>> const & items);
  temper::core::NullableParam<temper::core::Str> firstName() const;
  std::shared_ptr<std::vector<T>> get_items() const;
};

template <class T>
std::shared_ptr<Box<T>> Box<T>::make(T value) {
  global_init_init();
  std::shared_ptr<Box<T>> self = std::make_shared<Box<T>>();
  self->value = value;
  return self;
}

template <class T>
T Box<T>::get_held() const {
  return this->value;
}

template <class T>
std::shared_ptr<temper::core::Pair<T, int32_t>> Box<T>::pairWith(int32_t other) const {
  return temper::core::PairFactory::make<T, int32_t>(this->value, other);
}

template <class T>
T Box<T>::get_value() const {
  return this->value;
}

template <class T>
std::shared_ptr<Shelf<T>> Shelf<T>::make(std::shared_ptr<std::vector<T>> const & items) {
  global_init_init();
  std::shared_ptr<Shelf<T>> self = std::make_shared<Shelf<T>>();
  self->items = items;
  return self;
}

template <class T>
temper::core::NullableParam<temper::core::Str> Shelf<T>::firstName() const {
  if (temper::core::List::isEmpty<T>(this->items)) {
    return nullptr;
  } else {
    return temper::core::List::get<T>(this->items, 0)->name;
  }
}

template <class T>
std::shared_ptr<std::vector<T>> Shelf<T>::get_items() const {
  return this->items;
}

template <class T>
T first(std::shared_ptr<std::vector<T>> const & items, T fallback) {
  global_init_init();
  if (temper::core::List::isEmpty<T>(items)) {
    return fallback;
  } else {
    return temper::core::List::get<T>(items, 0);
  }
}

}  // namespace generics

// init.cpp
#include <generics/init.hpp>

namespace generics {

namespace core = temper::core;

std::shared_ptr<Named> Named::make(core::Str const & name) {
  global_init_init();
  std::shared_ptr<Named> self = std::make_shared<Named>();
  self->name = name;
  return self;
}

core::Str Named::get_name() const {
  return this->name;
}

template <class T>
static T held(std::shared_ptr<Holder<T>> const & h) {
  return h->get_held();
}

static int32_t total(std::shared_ptr<std::vector<int32_t>> const & ns) {
  int32_t sum = 0;
  {
    int32_t i = 0;
    while (i < core::List::length<int32_t>(ns)) {
      sum = core::Int::add(sum, core::List::get<int32_t>(ns, i));
      i = core::Int::add(i, 1);
    }
  }
  return sum;
}

static bool firstSame(std::shared_ptr<std::vector<int32_t>> const & a, std::shared_ptr<std::vector<int32_t>> const & b) {
  return core::List::get<int32_t>(a, 0) == core::List::get<int32_t>(b, 0);
}

void global_init_init() {
  core::init_once([] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    std::shared_ptr<std::vector<int32_t>> ns = core::List::make<int32_t>(1, 2, 3);
    std::shared_ptr<std::vector<int32_t>> none = core::List::make<int32_t>();
    core::Console::log(console, std::to_string(generics::total(ns)) + " " + std::to_string(generics::first<int32_t>(ns, 0)) + " " + std::to_string(generics::first<int32_t>(none, -1)));
    core::Console::log(console, generics::first<core::Str>(core::List::make<core::Str>(TEMPER_STR("a"), TEMPER_STR("b")), TEMPER_STR("z")));
    std::shared_ptr<Box<core::Str>> box = Box<core::Str>::make(TEMPER_STR("boxed"));
    core::Console::log(console, generics::held<core::Str>(box));
    std::shared_ptr<core::Pair<core::Str, int32_t>> pair = box->pairWith(7);
    core::Console::log(console, pair->key + " " + std::to_string(pair->value));
    std::shared_ptr<Shelf<std::shared_ptr<Named>>> shelf = Shelf<std::shared_ptr<Named>>::make(core::List::make<std::shared_ptr<Named>>(Named::make(TEMPER_STR("ann")), Named::make(TEMPER_STR("bo"))));
    core::NullableParam<core::Str> subject = shelf->firstName();
    core::Console::log(console, subject == nullptr ? TEMPER_STR("none") : subject.value);
    std::shared_ptr<std::vector<std::shared_ptr<Named>>> noNames = core::List::make<std::shared_ptr<Named>>();
    std::shared_ptr<Shelf<std::shared_ptr<Named>>> empty = Shelf<std::shared_ptr<Named>>::make(noNames);
    core::NullableParam<core::Str> subject2 = empty->firstName();
    core::Console::log(console, subject2 == nullptr ? TEMPER_STR("none") : subject2.value);
    std::shared_ptr<std::vector<core::Str>> builder = core::ListBuilder::make<core::Str>();
    core::ListBuilder::add<core::Str>(builder, TEMPER_STR("x"));
    core::ListBuilder::add<core::Str>(builder, TEMPER_STR("y"));
    core::ListBuilder::add<core::Str>(builder, TEMPER_STR("w"), 0);
    int32_t length = core::List::length<core::Str>(builder);
    core::Str get = core::List::get<core::Str>(builder, 0);
    core::Console::log(console, std::to_string(length) + " " + get + " " + core::ListBuilder::removeLast<core::Str>(builder));
    std::shared_ptr<std::vector<core::Str>> built = core::List::toList<core::Str>(builder);
    int32_t int32 = core::List::length<core::Str>(built);
    core::Str string = core::List::getOr<core::Str>(built, 5, TEMPER_STR("?"));
    core::Console::log(console, std::to_string(int32) + " " + string + " " + core::List::get<core::Str>(core::List::slice<core::Str>(built, 1, 2), 0));
    std::shared_ptr<core::Pair<core::Str, int32_t>> value2 = core::PairFactory::make<core::Str, int32_t>(TEMPER_STR("a"), 1);
    std::shared_ptr<core::Mapped::Ordered<core::Str, int32_t>> m = core::Map::make<core::Str, int32_t>(core::List::make<std::shared_ptr<core::Pair<core::Str, int32_t>>>(value2, core::PairFactory::make<core::Str, int32_t>(TEMPER_STR("b"), 2)));
    int32_t value3;
    try {
      value3 = core::Mapped::get<core::Str, int32_t>(m, TEMPER_STR("b"));
    } catch (core::TemperBubble const &) {
      value3 = -1;
    }
    int32_t value4;
    try {
      value4 = core::Mapped::get<core::Str, int32_t>(m, TEMPER_STR("c"));
    } catch (core::TemperBubble const &) {
      value4 = -1;
    }
    core::Console::log(console, std::to_string(value3) + " " + std::to_string(value4) + " " + core::Boolean::toString(core::Mapped::has<core::Str, int32_t>(m, TEMPER_STR("a"))) + " " + std::to_string(core::Mapped::length<core::Str, int32_t>(m)));
    std::shared_ptr<core::Mapped::Ordered<core::Str, core::NullableParam<int32_t>>> mb = core::Map::make<core::Str, core::NullableParam<int32_t>>();
    core::MapBuilder::set<core::Str, core::NullableParam<int32_t>>(mb, TEMPER_STR("q"), 5);
    core::MapBuilder::set<core::Str, core::NullableParam<int32_t>>(mb, TEMPER_STR("n"), nullptr);
    core::NullableParam<int32_t> subject3 = core::Mapped::getOr<core::Str, core::NullableParam<int32_t>>(mb, TEMPER_STR("q"), 0);
    core::NullableParam<int32_t> subject4 = core::Mapped::getOr<core::Str, core::NullableParam<int32_t>>(mb, TEMPER_STR("n"), 0);
    core::Console::log(console, std::to_string(subject3 == nullptr ? -2 : subject3.value) + " " + std::to_string(subject4 == nullptr ? -2 : subject4.value) + " " + core::Boolean::toString(core::Mapped::has<core::Str, core::NullableParam<int32_t>>(mb, TEMPER_STR("n"))));
    core::NullableParam<int32_t> subject6;
    try {
      subject6 = core::MapBuilder::remove<core::Str, core::NullableParam<int32_t>>(mb, TEMPER_STR("q"));
    } catch (core::TemperBubble const &) {
      subject6 = 0;
    }
    core::NullableParam<core::Str> subject5 = subject6 == nullptr ? nullptr : core::NullableParam<core::Str>(std::to_string(subject6.value));
    core::Console::log(console, subject5 == nullptr ? TEMPER_STR("null") : subject5.value);
    core::Console::log(console, std::to_string(core::List::length<core::Str>(core::Mapped::keys<core::Str, core::NullableParam<int32_t>>(mb))));
    core::Console::log(console, core::Boolean::toString(generics::firstSame(core::List::make<int32_t>(1000), core::List::make<int32_t>(1000))) + " " + core::Boolean::toString(generics::firstSame(core::List::make<int32_t>(1000), core::List::make<int32_t>(7))));
  });
}

}  // namespace generics
