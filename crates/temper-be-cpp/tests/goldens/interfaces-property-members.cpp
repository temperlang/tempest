// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace property_members {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

struct I;
struct J;
struct C;
template <class T>
struct D;
template <class T>
struct SI;
template <class T>
struct SSub;
struct AllAlone;
struct High;
struct Low1;
struct Mid;
struct Low2;
struct Low3;

struct I : public virtual temper::core::AnyValueBase {
  virtual std::string get_x() = 0;
};

struct J : public virtual I {};

struct C : public virtual J {
  std::string x;
  static std::shared_ptr<C> make(std::string x);
  std::string get_x() override;
};

template <class T>
struct D : public std::enable_shared_from_this<D<T>> {
  T thing;
  static std::shared_ptr<D<T>> make(T thing);
  T get_thing() const;
};

template <class T>
struct SI : public virtual temper::core::AnyValueBase {
  virtual T get_thing() const = 0;
};

template <class T>
struct SSub : public virtual SI<T> {
  T thing;
  static std::shared_ptr<SSub<T>> make(T thing);
  T get_thing() const override;
};

struct AllAlone : public virtual I {};

struct High : public virtual temper::core::AnyValueBase {
  virtual std::string toString() const = 0;
  virtual bool eq(std::shared_ptr<High> const & other) const = 0;
};

struct Low1 : public virtual High {
  int32_t i;
  static std::shared_ptr<Low1> make(int32_t i);
  std::string toString() const override;
  bool eq(std::shared_ptr<High> const & other) const override;
  int32_t get_i() const;
};

struct Mid : public virtual High {};

struct Low2 : public virtual Mid {
  std::string s;
  static std::shared_ptr<Low2> make(std::string s);
  std::string toString() const override;
  bool eq(std::shared_ptr<High> const & other) const override;
  std::string get_s() const;
};

struct Low3 : public virtual Mid {
  static std::shared_ptr<Low3> make();
  std::string toString() const override;
  bool eq(std::shared_ptr<High> const & other) const override;
};

template <class T>
std::shared_ptr<D<T>> D<T>::make(T thing) {
  std::shared_ptr<D<T>> self = std::make_shared<D<T>>();
  self->thing = thing;
  return self;
}

template <class T>
T D<T>::get_thing() const {
  return this->thing;
}

template <class T>
std::shared_ptr<SSub<T>> SSub<T>::make(T thing) {
  std::shared_ptr<SSub<T>> self = std::make_shared<SSub<T>>();
  self->thing = thing;
  return self;
}

template <class T>
T SSub<T>::get_thing() const {
  return this->thing;
}

}  // namespace property_members

// init.cpp
#include <property-members/init.hpp>

namespace property_members {

namespace core = temper::core;

template <class IT>
static IT leastX(IT a, IT b) {
  if (a->get_x() < b->get_x()) {
    return a;
  } else {
    return b;
  }
}

std::shared_ptr<C> C::make(std::string x) {
  std::shared_ptr<C> self = std::make_shared<C>();
  self->x = x;
  return self;
}

std::string C::get_x() {
  return this->x;
}

std::shared_ptr<Low1> Low1::make(int32_t i) {
  std::shared_ptr<Low1> self = std::make_shared<Low1>();
  self->i = i;
  return self;
}

std::string Low1::toString() const {
  return "Low1(" + std::to_string(this->i) + ")";
}

bool Low1::eq(std::shared_ptr<High> const & other) const {
  if (dynamic_cast<Low1 const *>(other.get()) != nullptr) {
    return this->i == core::checked_cast<Low1>(other)->i;
  } else {
    return false;
  }
}

int32_t Low1::get_i() const {
  return this->i;
}

std::shared_ptr<Low2> Low2::make(std::string s) {
  std::shared_ptr<Low2> self = std::make_shared<Low2>();
  self->s = s;
  return self;
}

std::string Low2::toString() const {
  return "Low2(" + this->s + ")";
}

bool Low2::eq(std::shared_ptr<High> const & other) const {
  if (dynamic_cast<Low2 const *>(other.get()) != nullptr) {
    return this->s == core::checked_cast<Low2>(other)->s;
  } else {
    return false;
  }
}

std::string Low2::get_s() const {
  return this->s;
}

std::shared_ptr<Low3> Low3::make() {
  std::shared_ptr<Low3> self = std::make_shared<Low3>();
  return self;
}

std::string Low3::toString() const {
  return "Low3";
}

bool Low3::eq(std::shared_ptr<High> const & other) const {
  return dynamic_cast<Low3 const *>(other.get()) != nullptr;
}

void global_init_init() {
  core::init_once([] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    std::shared_ptr<C> a = C::make("foo");
    std::shared_ptr<C> b = C::make("bar");
    core::Console::log(console, "leastX of " + a->x + " and " + b->x + " is " + leastX<std::shared_ptr<C>>(a, b)->x + ".");
    std::shared_ptr<I> i = a;
    core::Console::log(console, "As an I, a.x is " + i->get_x() + ".");
    std::shared_ptr<D<std::shared_ptr<I>>> di = D<std::shared_ptr<I>>::make(a);
    std::shared_ptr<D<std::shared_ptr<J>>> dj = D<std::shared_ptr<J>>::make(a);
    core::Console::log(console, "Generically, x is " + di->thing->get_x() + ".");
    core::Console::log(console, "Genericallier, x is still " + std::shared_ptr<I>(dj->thing)->get_x() + ".");
    std::shared_ptr<SI<std::shared_ptr<C>>> si = SSub<std::shared_ptr<C>>::make(a);
    core::Console::log(console, "Sealed thing is " + si->get_thing()->x + ".");
    std::shared_ptr<High> high1 = Low1::make(11);
    std::shared_ptr<High> high1b = Low1::make(12);
    std::shared_ptr<High> high1c = Low1::make(12);
    std::shared_ptr<High> high2 = Low2::make("two");
    std::shared_ptr<High> high2b = Low2::make("to be");
    std::shared_ptr<High> high2c = Low2::make("to be");
    std::shared_ptr<High> high3 = Low3::make();
    std::shared_ptr<High> high3b = Low3::make();
    std::shared_ptr<core::Pair<std::shared_ptr<High>, std::shared_ptr<High>>> value = core::PairFactory::make<std::shared_ptr<High>, std::shared_ptr<High>>(high1, high1b);
    std::shared_ptr<core::Pair<std::shared_ptr<High>, std::shared_ptr<High>>> value2 = core::PairFactory::make<std::shared_ptr<High>, std::shared_ptr<High>>(high1b, high1c);
    std::shared_ptr<core::Pair<std::shared_ptr<High>, std::shared_ptr<High>>> value3 = core::PairFactory::make<std::shared_ptr<High>, std::shared_ptr<High>>(high1, high2);
    std::shared_ptr<core::Pair<std::shared_ptr<High>, std::shared_ptr<High>>> value4 = core::PairFactory::make<std::shared_ptr<High>, std::shared_ptr<High>>(high2, high2b);
    std::shared_ptr<core::Pair<std::shared_ptr<High>, std::shared_ptr<High>>> value5 = core::PairFactory::make<std::shared_ptr<High>, std::shared_ptr<High>>(high2b, high2c);
    std::shared_ptr<core::Pair<std::shared_ptr<High>, std::shared_ptr<High>>> value6 = core::PairFactory::make<std::shared_ptr<High>, std::shared_ptr<High>>(high1, high3);
    std::shared_ptr<std::vector<std::shared_ptr<core::Pair<std::shared_ptr<High>, std::shared_ptr<High>>>>> pairs = core::List::make<std::shared_ptr<core::Pair<std::shared_ptr<High>, std::shared_ptr<High>>>>(value, value2, value3, value4, value5, value6, core::PairFactory::make<std::shared_ptr<High>, std::shared_ptr<High>>(high3, high3b));
    {
      int32_t length = core::List::length<std::shared_ptr<core::Pair<std::shared_ptr<High>, std::shared_ptr<High>>>>(pairs);
      int32_t i2 = 0;
      while (i2 < length) {
        std::shared_ptr<core::Pair<std::shared_ptr<High>, std::shared_ptr<High>>> pair = core::List::get<std::shared_ptr<core::Pair<std::shared_ptr<High>, std::shared_ptr<High>>>>(pairs, i2);
        i2 = core::Int::add(i2, 1);
        core::Console::log(console, pair->key->toString() + " == " + pair->value->toString() + ": " + core::Boolean::toString(pair->key->eq(pair->value)));
      }
    }
  });
}

}  // namespace property_members
