// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace libraries {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

struct Grid;
struct Casing;

struct Grid : public std::enable_shared_from_this<Grid> {
  int32_t ncols;
  std::shared_ptr<std::vector<double>> values;
  static std::shared_ptr<Grid> make(int32_t ncols, std::shared_ptr<std::vector<double>> const & values);
  double get(int32_t i, int32_t j) const;
  static std::shared_ptr<Grid> parse(std::string s);
  int32_t get_ncols() const;
  std::shared_ptr<std::vector<double>> get_values() const;
};

struct Casing : public std::enable_shared_from_this<Casing> {
  struct temper_statics {
    static temper::core::GlobalSlot<std::shared_ptr<Casing>> first;
  };
  std::string name;
  static std::shared_ptr<Casing> const & first();
  bool eq(std::shared_ptr<Casing> const & other) const;
  static std::shared_ptr<Casing> make(std::string name);
  std::string toString() const;
  std::string get_name() const;
};

std::shared_ptr<Grid> parseGrid(std::string s);

}  // namespace libraries

// init.cpp
#include <libraries/init.hpp>

namespace libraries {

namespace core = temper::core;

std::shared_ptr<Grid> Grid::make(int32_t ncols, std::shared_ptr<std::vector<double>> const & values) {
  global_init_init();
  std::shared_ptr<Grid> self = std::make_shared<Grid>();
  self->ncols = ncols;
  self->values = values;
  return self;
}

double Grid::get(int32_t i, int32_t j) const {
  return core::List::get<double>(this->values, core::Int::add(core::Int::mul(i, this->ncols), j));
}

std::shared_ptr<Grid> Grid::parse(std::string s) {
  global_init_init();
  try {
    return libraries::parseGrid(s);
  } catch (core::TemperBubble const &) {
    return nullptr;
  }
}

int32_t Grid::get_ncols() const {
  return this->ncols;
}

std::shared_ptr<std::vector<double>> Grid::get_values() const {
  return this->values;
}

std::shared_ptr<Grid> parseGrid(std::string s) {
  global_init_init();
  if (s == "") {
    core::bubble();
  }
  return Grid::make(1, core::List::make<double>(core::String::toFloat64(s)));
}

core::GlobalSlot<std::shared_ptr<Casing>> Casing::temper_statics::first;

std::shared_ptr<Casing> const & Casing::first() {
  global_init_init();
  return Casing::temper_statics::first.value;
}

bool Casing::eq(std::shared_ptr<Casing> const & other) const {
  return this->name == other->name;
}

std::shared_ptr<Casing> Casing::make(std::string name) {
  std::shared_ptr<Casing> self = std::make_shared<Casing>();
  self->name = name;
  return self;
}

std::string Casing::toString() const {
  return this->name;
}

std::string Casing::get_name() const {
  return this->name;
}

static int32_t rows(std::shared_ptr<std::vector<double>> const & values, int32_t ncols) {
  try {
    return core::Int::div_wrap(core::List::length<double>(values), ncols);
  } catch (core::TemperBubble const &) {
    core::panic();
  }
}

static std::shared_ptr<Grid> scaled(std::shared_ptr<Grid> const & grid) {
  if (grid->ncols == 0) {
    core::bubble();
  }
  return grid;
}

static std::shared_ptr<std::vector<double>> cubes(std::shared_ptr<Grid> const & grid) {
  try {
    std::shared_ptr<std::vector<double>> more = core::List::map(libraries::scaled(grid)->values, [](double x) -> double { return x * x * x; });
    return more;
  } catch (core::TemperBubble const &) {
    core::panic();
  }
}

static void each(std::shared_ptr<std::vector<double>> const & values, std::function<void(double)> const & f) {
  {
    int32_t i = 0;
    while (i < core::List::length<double>(values)) {
      f(core::List::get<double>(values, i));
      i = core::Int::add(i, 1);
    }
  }
}

static double total(std::shared_ptr<std::vector<double>> const & values) {
  std::shared_ptr<double> sum = std::make_shared<double>(0.0);
  libraries::each(values, [=](double x) -> void { try { double part = x / 2.0; *sum = (*sum) + part; } catch (core::TemperBubble const &) { core::panic(); } });
  return (*sum);
}

static double half(double x) {
  try {
    return x / 2.0;
  } catch (core::TemperBubble const &) {
    core::panic();
  }
}

static double cube(double x) {
  return x * 0.25;
}

static double corner(std::shared_ptr<Grid> const & grid) {
  try {
    return grid->get(1, 1);
  } catch (core::TemperBubble const &) {
    return -1.0;
  }
}

static std::shared_ptr<Grid> maybe(int32_t n) {
  if (n > 0) {
    return Grid::make(n, core::List::make<double>(1.0));
  } else {
    return nullptr;
  }
}

static std::shared_ptr<std::vector<double>> mapped(std::shared_ptr<Grid> const & grid) {
  std::shared_ptr<Grid> value;
  try {
    value = libraries::scaled(grid);
  } catch (core::TemperBubble const &) {
    core::panic();
  }
  return core::List::map(value->values, [](double x) -> double { if (core::Float64::gt(x, 1.5)) { return x * 2.0; } else { return x; } });
}

static core::NullableParam<std::string> text(int32_t n) {
  if (n > 0) {
    return "x";
  } else {
    return nullptr;
  }
}

static core::NullableParam<int32_t> count(int32_t n) {
  if (n > 0) {
    return n;
  } else {
    return nullptr;
  }
}

static std::shared_ptr<Casing> casing(int32_t n) {
  if (n > 0) {
    return Casing::temper_statics::first.value;
  } else {
    return nullptr;
  }
}

static std::string captured(std::string s) {
  std::shared_ptr<Grid> found;
  try {
    found = libraries::parseGrid(s);
  } catch (core::TemperBubble const &) {
    found = nullptr;
  }
  return core::List::join(core::List::map(core::List::make<int32_t>(1, 2), [=](int32_t) -> bool { return found == nullptr; }), ",", [](bool b) -> std::string { return core::Boolean::toString(b); });
}

static std::string bidi() {
  return "look\u202Eout";
}

static std::string word(int32_t n) {
  if (n > 1) {
    return "b";
  } else {
    return "a";
  }
}

void global_init_init() {
  core::init_once([] {
    Casing::temper_statics::first.construct();
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    Casing::temper_statics::first.value = Casing::make("first");
    std::shared_ptr<Grid> grid = Grid::make(2, core::List::make<double>(1.0, 2.0, 3.0, 4.0));
    int32_t int32 = libraries::rows(grid->values, 2);
    core::Console::log(console, std::to_string(int32) + " " + core::Float64::toString(half(3.0)) + " " + core::Float64::toString(cube(8.0)));
    core::Console::log(console, core::Float64::toString(libraries::corner(grid)));
    core::Console::log(console, core::Float64::toString(libraries::total(grid->values)));
    core::NullableParam<std::string> none = text(0);
    core::NullableParam<int32_t> some = count(3);
    core::NullableParam<std::string> part = none;
    core::NullableParam<int32_t> part2 = some;
    std::shared_ptr<Grid> subject = maybe(0);
    core::NullableParam<int32_t> part3 = subject == nullptr ? nullptr : core::NullableParam<int32_t>(subject->ncols);
    core::Console::log(console, (part == nullptr ? "null" : part.value) + " " + (part2 == nullptr ? "null" : std::to_string(part2.value)) + " " + (part3 == nullptr ? "null" : std::to_string(part3.value)));
    core::Console::log(console, core::Boolean::toString(maybe(1) != nullptr) + " " + core::Boolean::toString(maybe(0) != nullptr) + " " + core::Boolean::toString(Grid::parse("") == nullptr));
    core::Console::log(console, Casing::temper_statics::first.value->toString());
    core::Console::log(console, core::List::join(libraries::mapped(grid), ",", [](double x) -> std::string { return core::Float64::toString(x); }));
    core::Console::log(console, core::List::join(libraries::cubes(grid), ",", [](double x) -> std::string { return core::Float64::toString(x); }));
    std::shared_ptr<Casing> left = casing(1);
    std::shared_ptr<Casing> left2 = casing(0);
    std::shared_ptr<Casing> left3 = casing(0);
    std::shared_ptr<Casing> right = casing(0);
    core::Console::log(console, core::Boolean::toString(left == nullptr ? false : left->eq(Casing::temper_statics::first.value)) + " " + core::Boolean::toString(left2 == nullptr ? false : left2->eq(Casing::temper_statics::first.value)) + " " + core::Boolean::toString(!(left3 == nullptr ? right == nullptr : right == nullptr ? false : left3->eq(right))));
    std::string string = libraries::captured("");
    core::Console::log(console, string + " " + libraries::captured("2.5"));
    core::Console::log(console, core::Boolean::toString(bidi() != "lookout"));
    core::Console::log(console, std::to_string(core::Compare::cmp(word(1), word(2))) + " " + std::to_string(core::Compare::cmp(word(2), word(1))) + " " + std::to_string(core::Compare::cmp(word(2), word(3))));
    std::string named = word(2);
    core::Console::log(console, core::Boolean::toString(named == word(3) && named + "!" != named));
  });
}

}  // namespace libraries
