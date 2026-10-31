// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace generics {

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
  std::string name;
  static std::shared_ptr<Named> make(std::string name);
  std::string get_name() const;
};

template <class T>
struct Shelf : public std::enable_shared_from_this<Shelf<T>> {
  std::shared_ptr<std::vector<T>> items;
  static std::shared_ptr<Shelf<T>> make(std::shared_ptr<std::vector<T>> const & items);
  temper::core::NullableParam<std::string> firstName() const;
  std::shared_ptr<std::vector<T>> get_items() const;
};

template <class T>
std::shared_ptr<Box<T>> Box<T>::make(T value) {
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
  std::shared_ptr<Shelf<T>> self = std::make_shared<Shelf<T>>();
  self->items = items;
  return self;
}

template <class T>
temper::core::NullableParam<std::string> Shelf<T>::firstName() const {
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
  if (temper::core::List::isEmpty<T>(items)) {
    return fallback;
  } else {
    return temper::core::List::get<T>(items, 0);
  }
}

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace generics

// init.cpp
#include <generics/init.hpp>

#include <mutex>

namespace generics {

namespace core = temper::core;

std::shared_ptr<Named> Named::make(std::string name) {
  std::shared_ptr<Named> self = std::make_shared<Named>();
  self->name = name;
  return self;
}

std::string Named::get_name() const {
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

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    std::shared_ptr<std::vector<int32_t>> ns = core::List::make<int32_t>(1, 2, 3);
    std::shared_ptr<std::vector<int32_t>> none = core::List::make<int32_t>();
    int32_t int32 = total(ns);
    int32_t value2 = first<int32_t>(ns, 0);
    core::Console::log(console, std::to_string(int32) + " " + std::to_string(value2) + " " + std::to_string(first<int32_t>(none, -1)));
    core::Console::log(console, first<std::string>(core::List::make<std::string>("a", "b"), "z"));
    std::shared_ptr<Box<std::string>> box = Box<std::string>::make("boxed");
    core::Console::log(console, held<std::string>(box));
    std::shared_ptr<core::Pair<std::string, int32_t>> pair = box->pairWith(7);
    core::Console::log(console, pair->key + " " + std::to_string(pair->value));
    std::shared_ptr<Shelf<std::shared_ptr<Named>>> shelf = Shelf<std::shared_ptr<Named>>::make(core::List::make<std::shared_ptr<Named>>(Named::make("ann"), Named::make("bo")));
    core::NullableParam<std::string> subject = shelf->firstName();
    core::Console::log(console, subject == nullptr ? "none" : subject.value);
    std::shared_ptr<std::vector<std::shared_ptr<Named>>> noNames = core::List::make<std::shared_ptr<Named>>();
    std::shared_ptr<Shelf<std::shared_ptr<Named>>> empty = Shelf<std::shared_ptr<Named>>::make(noNames);
    core::NullableParam<std::string> subject2 = empty->firstName();
    core::Console::log(console, subject2 == nullptr ? "none" : subject2.value);
    std::shared_ptr<std::vector<std::string>> builder = core::ListBuilder::make<std::string>();
    core::ListBuilder::add<std::string>(builder, "x");
    core::ListBuilder::add<std::string>(builder, "y");
    core::ListBuilder::add<std::string>(builder, "w", 0);
    int32_t length = core::List::length<std::string>(builder);
    std::string get = core::List::get<std::string>(builder, 0);
    core::Console::log(console, std::to_string(length) + " " + get + " " + core::ListBuilder::removeLast<std::string>(builder));
    std::shared_ptr<std::vector<std::string>> built = core::List::toList<std::string>(builder);
    int32_t value3 = core::List::length<std::string>(built);
    std::string string = core::List::getOr<std::string>(built, 5, "?");
    core::Console::log(console, std::to_string(value3) + " " + string + " " + core::List::get<std::string>(core::List::slice<std::string>(built, 1, 2), 0));
    std::shared_ptr<core::Pair<std::string, int32_t>> value4 = core::PairFactory::make<std::string, int32_t>("a", 1);
    std::shared_ptr<core::Mapped::Ordered<std::string, int32_t>> m = core::Map::make<std::string, int32_t>(core::List::make<std::shared_ptr<core::Pair<std::string, int32_t>>>(value4, core::PairFactory::make<std::string, int32_t>("b", 2)));
    int32_t value5;
    try {
      value5 = core::Mapped::get<std::string, int32_t>(m, "b");
    } catch (core::TemperBubble const &) {
      value5 = -1;
    }
    int32_t value6 = value5;
    int32_t value7;
    try {
      value7 = core::Mapped::get<std::string, int32_t>(m, "c");
    } catch (core::TemperBubble const &) {
      value7 = -1;
    }
    int32_t value8 = value7;
    bool has = core::Mapped::has<std::string, int32_t>(m, "a");
    core::Console::log(console, std::to_string(value6) + " " + std::to_string(value8) + " " + core::Boolean::toString(has) + " " + std::to_string(core::Mapped::length<std::string, int32_t>(m)));
    std::shared_ptr<core::Mapped::Ordered<std::string, core::NullableParam<int32_t>>> mb = core::Map::make<std::string, core::NullableParam<int32_t>>();
    core::MapBuilder::set<std::string, core::NullableParam<int32_t>>(mb, "q", 5);
    core::MapBuilder::set<std::string, core::NullableParam<int32_t>>(mb, "n", nullptr);
    core::NullableParam<int32_t> subject3 = core::Mapped::getOr<std::string, core::NullableParam<int32_t>>(mb, "q", 0);
    int32_t value9 = subject3 == nullptr ? -2 : subject3.value;
    core::NullableParam<int32_t> subject4 = core::Mapped::getOr<std::string, core::NullableParam<int32_t>>(mb, "n", 0);
    int32_t value10 = subject4 == nullptr ? -2 : subject4.value;
    core::Console::log(console, std::to_string(value9) + " " + std::to_string(value10) + " " + core::Boolean::toString(core::Mapped::has<std::string, core::NullableParam<int32_t>>(mb, "n")));
    core::NullableParam<int32_t> subject6;
    try {
      subject6 = core::MapBuilder::remove<std::string, core::NullableParam<int32_t>>(mb, "q");
    } catch (core::TemperBubble const &) {
      subject6 = 0;
    }
    core::NullableParam<std::string> subject5 = subject6 == nullptr ? nullptr : core::NullableParam<std::string>(std::to_string(subject6.value));
    core::Console::log(console, subject5 == nullptr ? "null" : subject5.value);
    core::Console::log(console, std::to_string(core::List::length<std::string>(core::Mapped::keys<std::string, core::NullableParam<int32_t>>(mb))));
  });
}

}  // namespace generics
