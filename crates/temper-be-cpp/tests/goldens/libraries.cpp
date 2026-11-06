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
struct Shape;
struct Dot;
struct Line;
struct Carry;
struct Tagged;

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
    static temper::core::GlobalSlot<std::shared_ptr<Casing>> last;
  };
  std::string name;
  static std::shared_ptr<Casing> const & first();
  static std::shared_ptr<Casing> const & last();
  bool eq(std::shared_ptr<Casing> const & other) const;
  static std::shared_ptr<Casing> make(std::string name);
  std::string toString() const;
  static std::shared_ptr<Casing> named(std::string name);
  std::string get_name() const;
};

struct Shape : public virtual temper::core::AnyValueBase {
  virtual int32_t get_size() const = 0;
};

struct Dot : public virtual Shape {
  int32_t size;
  static std::shared_ptr<Dot> make(int32_t size);
  std::shared_ptr<Dot> get_bigger() const;
  temper::core::NullableParam<int32_t> get_grown() const;
  int32_t scaled(int32_t k) const;
  int32_t get_size() const override;
};

struct Line : public virtual Shape {
  int32_t size;
  static std::shared_ptr<Line> make(int32_t size);
  int32_t get_size() const override;
};

struct Carry : public std::enable_shared_from_this<Carry> {
  bool carrying;
  int32_t state;
  static std::shared_ptr<Carry> make(int32_t start);
  bool get_carrying() const;
  int32_t get_state() const;
};

struct Tagged : public std::enable_shared_from_this<Tagged> {
  temper::core::NullableParam<std::string> label;
  static std::shared_ptr<Tagged> make(temper::core::NullableParam<std::string> label);
  temper::core::NullableParam<std::string> get_label() const;
};

// Storage, which the library reads and its init sets; read values
// through the accessors below.
namespace temper_globals {
extern temper::core::GlobalSlot<std::shared_ptr<std::vector<int32_t>>> extras;
}  // namespace temper_globals

std::shared_ptr<Grid> parseGrid(std::string s);

template <class T>
int32_t sizeWith(std::shared_ptr<std::vector<T>> const & items) {
  global_init_init();
  return temper::core::Int::add(temper::core::List::length<T>(items), temper::core::List::length<int32_t>(temper_globals::extras.value));
}

}  // namespace libraries

// init.cpp
#include <libraries/init.hpp>

#include <mutex>

namespace libraries {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<int32_t> ticks;
core::GlobalSlot<std::shared_ptr<std::vector<int32_t>>> extras;

}  // namespace temper_globals

static std::mutex moduleMutex;

// Reads a variable that functions write, under the module's lock.
template <class T>
static T load(T const &variable) {
  std::lock_guard<std::mutex> lock(moduleMutex);
  return variable;
}

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

core::GlobalSlot<std::shared_ptr<Casing>> Casing::temper_statics::last;

std::shared_ptr<Casing> const & Casing::last() {
  global_init_init();
  return Casing::temper_statics::last.value;
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

std::shared_ptr<Casing> Casing::named(std::string name) {
  return Casing::make(name);
}

std::string Casing::get_name() const {
  return this->name;
}

std::shared_ptr<Dot> Dot::make(int32_t size) {
  global_init_init();
  std::shared_ptr<Dot> self = std::make_shared<Dot>();
  self->size = size;
  return self;
}

std::shared_ptr<Dot> Dot::get_bigger() const {
  std::shared_ptr<Dot const> self = std::shared_ptr<Dot const>(shared_from_this(), this);
  if (self->size < 3) {
    return Dot::make(core::Int::add(self->size, 1));
  } else {
    return nullptr;
  }
}

core::NullableParam<int32_t> Dot::get_grown() const {
  std::shared_ptr<Dot const> self = std::shared_ptr<Dot const>(shared_from_this(), this);
  {
    std::shared_ptr<Dot> subject = self->get_bigger();
    if (subject == nullptr) {
      return nullptr;
    } else {
      return subject->scaled(10);
    }
  }
}

int32_t Dot::scaled(int32_t k) const {
  return core::Int::mul(this->size, k);
}

int32_t Dot::get_size() const {
  return this->size;
}

std::shared_ptr<Line> Line::make(int32_t size) {
  global_init_init();
  std::shared_ptr<Line> self = std::make_shared<Line>();
  self->size = size;
  return self;
}

int32_t Line::get_size() const {
  return this->size;
}

std::shared_ptr<Carry> Carry::make(int32_t start) {
  global_init_init();
  std::shared_ptr<Carry> self = std::make_shared<Carry>();
  self->carrying = false;
  self->state = 0;
  if (start != 0) {
    self->carrying = true;
    self->state = start;
  }
  return self;
}

bool Carry::get_carrying() const {
  return this->carrying;
}

int32_t Carry::get_state() const {
  return this->state;
}

std::shared_ptr<Tagged> Tagged::make(core::NullableParam<std::string> label) {
  global_init_init();
  std::shared_ptr<Tagged> self = std::make_shared<Tagged>();
  self->label = label;
  return self;
}

core::NullableParam<std::string> Tagged::get_label() const {
  return this->label;
}

static std::shared_ptr<Tagged> tagged(core::NullableParam<std::string> label) {
  return Tagged::make(label);
}

static std::shared_ptr<Tagged> tagged() {
  return tagged(nullptr);
}

static int32_t weight(int32_t c) {
  int32_t base = c == 42 ? 0 : 6;
  return core::Int::add(core::Int::add(c > 50 ? 1 : 2, base), c);
}

static std::string shapeName(std::shared_ptr<Shape> const & s) {
  std::string name = dynamic_cast<Dot const *>(s.get()) != nullptr ? "dot" : dynamic_cast<Line const *>(s.get()) != nullptr ? "line" : core::panic<std::string>();
  return name;
}

static std::shared_ptr<Shape> shapeOf(int32_t n) {
  std::shared_ptr<Shape> shape = n > 1 ? std::shared_ptr<Shape>(Line::make(n)) : std::shared_ptr<Shape>(Dot::make(n));
  return shape;
}

static std::shared_ptr<std::vector<int32_t>> sizes(std::shared_ptr<Shape> const & s) {
  std::shared_ptr<std::vector<int32_t>> found = dynamic_cast<Dot const *>(s.get()) != nullptr ? core::List::make<int32_t>(core::checked_cast<Dot>(s)->size, core::checked_cast<Dot>(s)->size) : core::List::make<int32_t>();
  return found;
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
  std::shared_ptr<Grid> value2;
  try {
    value2 = libraries::scaled(grid);
  } catch (core::TemperBubble const &) {
    core::panic();
  }
  return core::List::map(value2->values, [](double x) -> double { if (core::Float64::gt(x, 1.5)) { return x * 2.0; } else { return x; } });
}

static int32_t byteAt(std::shared_ptr<std::vector<int32_t>> const & digits, bool short_, int32_t c) {
  int32_t byte = short_ ? core::Int::mul(core::List::get<int32_t>(digits, c), 17) : core::Int::add(core::Int::mul(core::List::get<int32_t>(digits, core::Int::mul(2, c)), 16), core::List::get<int32_t>(digits, core::Int::add(core::Int::mul(2, c), 1)));
  return byte;
}

static bool ascending(std::shared_ptr<std::vector<int32_t>> const & digits) {
  if (core::List::length<int32_t>(digits) > 1) {
    return core::Int::add(core::List::get<int32_t>(digits, 0), 1) == core::List::get<int32_t>(digits, 1);
  } else {
    return false;
  }
}

static int32_t firstDrop(std::shared_ptr<std::vector<int32_t>> const & digits) {
  int32_t i = 1;
  while (i < core::List::length<int32_t>(digits) && core::List::get<int32_t>(digits, core::Int::sub(i, 1)) <= core::List::get<int32_t>(digits, i)) {
    i = core::Int::add(i, 1);
  }
  return i;
}

static int32_t tick() {
  std::lock_guard<std::mutex> lock(moduleMutex);
  temper_globals::ticks.value = core::Int::add(temper_globals::ticks.value, 1);
  return temper_globals::ticks.value;
}

static int32_t tickIf(bool short_) {
  int32_t int32;
  if (short_) {
    int32 = tick();
  } else {
    int32_t value2 = core::Int::mul(tick(), 10);
    int32 = core::Int::add(value2, tick());
  }
  int32_t n = int32;
  return n;
}

static bool ticked() {
  if (load(temper_globals::ticks.value) > 0) {
    int32_t value2 = core::Int::mul(tick(), 10);
    return core::Int::add(value2, tick()) > 0;
  } else {
    return false;
  }
}

static int32_t tickWhile() {
  int32_t i = 0;
  while (true) {
    bool boolean = i < 3;
    if (boolean) {
      int32_t value2 = tick();
      boolean = value2 < tick();
    }
    if (!boolean) {
      break;
    }
    i = core::Int::add(i, 1);
  }
  return i;
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
    temper_globals::ticks.construct();
    temper_globals::extras.construct();
    Casing::temper_statics::first.construct();
    Casing::temper_statics::last.construct();
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    Casing::temper_statics::first.value = Casing::make("first");
    Casing::temper_statics::last.value = Casing::named("last");
    temper_globals::ticks.value = 0;
    temper_globals::extras.value = core::List::make<int32_t>(1);
    std::shared_ptr<Grid> grid = Grid::make(2, core::List::make<double>(1.0, 2.0, 3.0, 4.0));
    int32_t value2 = libraries::rows(grid->values, 2);
    core::Console::log(console, std::to_string(value2) + " " + core::Float64::toString(half(3.0)) + " " + core::Float64::toString(cube(8.0)));
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
    std::string value3 = libraries::captured("");
    core::Console::log(console, value3 + " " + libraries::captured("2.5"));
    core::Console::log(console, core::Boolean::toString(bidi() != "lookout"));
    core::Console::log(console, std::to_string(core::Compare::cmp(word(1), word(2))) + " " + std::to_string(core::Compare::cmp(word(2), word(1))) + " " + std::to_string(core::Compare::cmp(word(2), word(3))));
    std::string named = word(2);
    core::Console::log(console, core::Boolean::toString(named == word(3) && named + "!" != named));
    std::shared_ptr<std::vector<int32_t>> digits = core::List::make<int32_t>(1, 2, 4, 3);
    core::Console::log(console, std::to_string(libraries::byteAt(digits, true, 1)) + " " + std::to_string(libraries::byteAt(digits, false, 0)) + " " + core::Boolean::toString(libraries::ascending(digits)) + " " + std::to_string(libraries::firstDrop(digits)) + " " + std::to_string(libraries::sizeWith<int32_t>(digits)));
    int32_t value4 = tickIf(true);
    int32_t value5 = tickIf(false);
    bool value6 = ticked();
    core::Console::log(console, std::to_string(value4) + " " + std::to_string(value5) + " " + core::Boolean::toString(value6) + " " + std::to_string(tickWhile()));
    std::string value7 = libraries::shapeName(shapeOf(1));
    std::string value8 = libraries::shapeName(shapeOf(2));
    std::string string = value7 + " " + value8 + " " + std::to_string(core::List::length<int32_t>(libraries::sizes(shapeOf(1)))) + " " + std::to_string(core::List::length<int32_t>(libraries::sizes(shapeOf(2)))) + " ";
    core::NullableParam<int32_t> part4 = Dot::make(1)->get_grown();
    core::NullableParam<int32_t> part5 = Dot::make(3)->get_grown();
    core::Console::log(console, string + (part4 == nullptr ? "null" : std::to_string(part4.value)) + " " + (part5 == nullptr ? "null" : std::to_string(part5.value)) + " " + Casing::temper_statics::last.value->toString());
    std::shared_ptr<Carry> carry = Carry::make(3);
    std::string value = core::Boolean::toString(carry->carrying) + " " + std::to_string(carry->state) + " " + core::Boolean::toString(Carry::make(0)->carrying) + " ";
    core::NullableParam<std::string> part6 = libraries::tagged()->label;
    core::NullableParam<std::string> part7 = libraries::tagged("x")->label;
    core::Console::log(console, value + (part6 == nullptr ? "null" : part6.value) + " " + (part7 == nullptr ? "null" : part7.value) + " " + std::to_string(weight(42)) + " " + std::to_string(weight(60)));
  });
}

}  // namespace libraries
