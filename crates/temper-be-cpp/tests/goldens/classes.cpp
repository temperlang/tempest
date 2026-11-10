// init.hpp
#pragma once

#include <temper-core/core.hpp>
#include <typeindex>

namespace classes {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

struct Shape;
struct Rect;
struct Square;
struct Counter;
struct Point;
struct Loose;
struct Holder;
struct Version;
struct Sql;

struct Shape : public virtual temper::core::AnyValueBase {
  virtual int32_t get_area() const = 0;
  virtual temper::core::Str describe() const;
};

struct Rect : public virtual Shape {
  struct temper_statics {
    static temper::core::GlobalSlot<std::shared_ptr<Rect>> unit;
  };
  int32_t w;
  int32_t h;
  static std::shared_ptr<Rect> const & unit();
  static std::shared_ptr<Rect> make(int32_t w, int32_t h);
  int32_t get_area() const override;
  temper::core::Str toString() const;
  std::shared_ptr<Rect> scaled(int32_t k) const;
  int32_t get_w() const;
  int32_t get_h() const;
};

struct Square : public virtual Shape {
  int32_t side;
  static std::shared_ptr<Square> make(int32_t side);
  int32_t get_area() const override;
  temper::core::Str describe() const override;
};

struct Counter : public std::enable_shared_from_this<Counter> {
  int32_t count;
  int32_t hidden;
  static std::shared_ptr<Counter> make();
  int32_t get_doubled();
  void set_doubled(int32_t value);
  std::shared_ptr<Counter> bump();
  bool same(std::shared_ptr<Counter> const & other) const;
  static std::shared_ptr<Counter> zero();
  int32_t get_count();
};

struct Point : public std::enable_shared_from_this<Point> {
  int32_t x;
  int32_t y;
  static std::shared_ptr<Point> make(int32_t x, int32_t y);
  bool eq(std::shared_ptr<Point> const & other) const;
  int32_t get_x() const;
  int32_t get_y() const;
};

struct Loose : public std::enable_shared_from_this<Loose> {
  int32_t n;
  static std::shared_ptr<Loose> make(int32_t n);
  bool eq(std::shared_ptr<Loose> const & other) const;
  int32_t get_n() const;
};

struct Holder : public std::enable_shared_from_this<Holder> {
  std::shared_ptr<Loose> loose;
  temper::core::Str tag;
  static std::shared_ptr<Holder> make(std::shared_ptr<Loose> const & loose, temper::core::Str const & tag);
  bool eq(std::shared_ptr<Holder> const & other) const;
  std::shared_ptr<Loose> get_loose() const;
  temper::core::Str get_tag() const;
};

struct Version : public std::enable_shared_from_this<Version> {
  int32_t major;
  int32_t minor;
  static std::shared_ptr<Version> make(int32_t major, int32_t minor);
  int32_t compareTo(std::shared_ptr<Version> const & other) const;
  int32_t get_major() const;
  int32_t get_minor() const;
};

struct Sql : public std::enable_shared_from_this<Sql> {
  std::shared_ptr<std::vector<temper::core::Str>> parts;
  static std::shared_ptr<Sql> make();
  void appendSafe(temper::core::Str const & s) const;
  void appendString(temper::core::NullableParam<temper::core::Str> x) const;
  void appendInt(int32_t x) const;
  void appendShape(std::shared_ptr<Shape> const & x) const;
  void appendRect(std::shared_ptr<Rect> const & x) const;
  temper::core::Str get_accumulated() const;
};

}  // namespace classes

// init.cpp
#include <classes/init.hpp>

namespace classes {

namespace core = temper::core;

core::Str Shape::describe() const {
  std::shared_ptr<Shape const> self = std::shared_ptr<Shape const>(shared_from_this(), this);
  return "shape of area " + std::to_string(self->get_area());
}

core::GlobalSlot<std::shared_ptr<Rect>> Rect::temper_statics::unit;

std::shared_ptr<Rect> const & Rect::unit() {
  global_init_init();
  return Rect::temper_statics::unit.value;
}

std::shared_ptr<Rect> Rect::make(int32_t w, int32_t h) {
  global_init_init();
  std::shared_ptr<Rect> self = std::make_shared<Rect>();
  self->w = w;
  self->h = h;
  return self;
}

int32_t Rect::get_area() const {
  return core::Int::mul(this->w, this->h);
}

core::Str Rect::toString() const {
  return "Rect(" + std::to_string(this->w) + ", " + std::to_string(this->h) + ")";
}

std::shared_ptr<Rect> Rect::scaled(int32_t k) const {
  std::shared_ptr<Rect const> self = std::shared_ptr<Rect const>(shared_from_this(), this);
  return Rect::make(core::Int::mul(self->w, k), core::Int::mul(self->h, k));
}

int32_t Rect::get_w() const {
  return this->w;
}

int32_t Rect::get_h() const {
  return this->h;
}

std::shared_ptr<Square> Square::make(int32_t side) {
  global_init_init();
  std::shared_ptr<Square> self = std::make_shared<Square>();
  self->side = side;
  return self;
}

int32_t Square::get_area() const {
  return core::Int::mul(this->side, this->side);
}

core::Str Square::describe() const {
  return "square of side " + std::to_string(this->side);
}

std::shared_ptr<Counter> Counter::make() {
  std::shared_ptr<Counter> self = std::make_shared<Counter>();
  self->count = 0;
  self->hidden = 0;
  return self;
}

int32_t Counter::get_doubled() {
  return core::Int::mul(this->count, 2);
}

void Counter::set_doubled(int32_t value) {
  this->count = core::Int::sub(value, this->count);
}

std::shared_ptr<Counter> Counter::bump() {
  std::shared_ptr<Counter> self = shared_from_this();
  self->count = core::Int::add(self->count, 1);
  self->hidden = core::Int::add(self->hidden, 1);
  return self;
}

bool Counter::same(std::shared_ptr<Counter> const & other) const {
  return this->hidden == other->hidden;
}

std::shared_ptr<Counter> Counter::zero() {
  return Counter::make();
}

int32_t Counter::get_count() {
  return this->count;
}

std::shared_ptr<Point> Point::make(int32_t x, int32_t y) {
  std::shared_ptr<Point> self = std::make_shared<Point>();
  self->x = x;
  self->y = y;
  return self;
}

bool Point::eq(std::shared_ptr<Point> const & other) const {
  if (this->x == other->x) {
    return this->y == other->y;
  } else {
    return false;
  }
}

int32_t Point::get_x() const {
  return this->x;
}

int32_t Point::get_y() const {
  return this->y;
}

std::shared_ptr<Loose> Loose::make(int32_t n) {
  std::shared_ptr<Loose> self = std::make_shared<Loose>();
  self->n = n;
  return self;
}

bool Loose::eq(std::shared_ptr<Loose> const & other) const {
  if (this->n == other->n) {
    return true;
  } else {
    return this->n == core::Int::neg(other->n);
  }
}

int32_t Loose::get_n() const {
  return this->n;
}

std::shared_ptr<Holder> Holder::make(std::shared_ptr<Loose> const & loose, core::Str const & tag) {
  std::shared_ptr<Holder> self = std::make_shared<Holder>();
  self->loose = loose;
  self->tag = tag;
  return self;
}

bool Holder::eq(std::shared_ptr<Holder> const & other) const {
  std::shared_ptr<Holder const> self = shared_from_this();
  if (self->loose->eq(other->loose)) {
    return self->tag == other->tag;
  } else {
    return false;
  }
}

std::shared_ptr<Loose> Holder::get_loose() const {
  return this->loose;
}

core::Str Holder::get_tag() const {
  return this->tag;
}

std::shared_ptr<Version> Version::make(int32_t major, int32_t minor) {
  std::shared_ptr<Version> self = std::make_shared<Version>();
  self->major = major;
  self->minor = minor;
  return self;
}

int32_t Version::compareTo(std::shared_ptr<Version> const & other) const {
  if (this->major != other->major) {
    return core::Compare::cmp(this->major, other->major);
  } else {
    return core::Compare::cmp(this->minor, other->minor);
  }
}

int32_t Version::get_major() const {
  return this->major;
}

int32_t Version::get_minor() const {
  return this->minor;
}

static std::shared_ptr<Shape> shape(std::shared_ptr<Shape> const & x) {
  return x;
}

static std::type_index typeNamed(int32_t n) {
  {
    if (n == 0) {
      return std::type_index(typeid(void));
    } else if (n == 1) {
      return std::type_index(typeid(bool));
    } else if (n == 2) {
      return std::type_index(typeid(int32_t));
    } else if (n == 3) {
      return std::type_index(typeid(int64_t));
    } else if (n == 4) {
      return std::type_index(typeid(double));
    } else {
      return std::type_index(typeid(core::Str));
    }
  }
}

static std::type_index same(std::type_index t) {
  return t;
}

std::shared_ptr<Sql> Sql::make() {
  std::shared_ptr<Sql> self = std::make_shared<Sql>();
  self->parts = core::ListBuilder::make<core::Str>();
  return self;
}

void Sql::appendSafe(core::Str const & s) const {
  core::ListBuilder::add<core::Str>(this->parts, s);
}

void Sql::appendString(core::NullableParam<core::Str> x) const {
  core::ListBuilder::add<core::Str>(this->parts, x == nullptr ? TEMPER_STR("NULL") : core::Str("'" + x.value + "'"));
}

void Sql::appendInt(int32_t x) const {
  core::ListBuilder::add<core::Str>(this->parts, std::to_string(x));
}

void Sql::appendShape(std::shared_ptr<Shape> const & x) const {
  std::shared_ptr<Sql const> self = shared_from_this();
  core::ListBuilder::add<core::Str>(self->parts, "shape " + std::to_string(x->get_area()));
}

void Sql::appendRect(std::shared_ptr<Rect> const & x) const {
  std::shared_ptr<Sql const> self = shared_from_this();
  core::ListBuilder::add<core::Str>(self->parts, "rect " + x->toString());
}

core::Str Sql::get_accumulated() const {
  return core::List::join(this->parts, core::Str(), [](core::Str p) -> core::Str { return p; });
}

void global_init_init() {
  core::init_once([] {
    Rect::temper_statics::unit.construct();
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    Rect::temper_statics::unit.value = Rect::make(1, 1);
    std::shared_ptr<Rect> r = Rect::make(2, 3);
    core::Console::log(console, r->toString());
    core::Console::log(console, std::to_string(r->get_area()));
    std::shared_ptr<Shape> s = classes::shape(core::to_base<Shape>(r));
    core::Console::log(console, s->describe());
    std::shared_ptr<Shape> q = classes::shape(core::to_base<Shape>(Square::make(4)));
    core::Console::log(console, q->describe());
    core::Console::log(console, std::to_string(q->get_area()));
    core::Console::log(console, std::to_string(r->scaled(2)->scaled(3)->get_area()));
    core::Console::log(console, Rect::temper_statics::unit.value->scaled(5)->toString());
    std::shared_ptr<Counter> c = Counter::zero();
    c->bump()->bump();
    core::Console::log(console, std::to_string(c->count));
    c->set_doubled(10);
    core::Console::log(console, std::to_string(c->get_doubled()));
    core::Console::log(console, core::Boolean::toString(c->same(Counter::make()->bump()->bump())));
    core::Console::log(console, core::Boolean::toString(Point::make(1, 2)->eq(Point::make(1, 2))) + " " + core::Boolean::toString(!Point::make(1, 2)->eq(Point::make(2, 1))));
    core::Console::log(console, core::Boolean::toString(Holder::make(Loose::make(3), TEMPER_STR("a"))->eq(Holder::make(Loose::make(-3), TEMPER_STR("a")))));
    core::Console::log(console, core::Boolean::toString(Version::make(1, 2)->compareTo(Version::make(1, 3)) < 0) + " " + core::Boolean::toString(Version::make(2, 0)->compareTo(Version::make(1, 9)) >= 0));
    core::Console::log(console, core::Boolean::toString(dynamic_cast<Square const *>(q.get()) != nullptr) + " " + core::Boolean::toString(dynamic_cast<Square const *>(s.get()) != nullptr));
    std::shared_ptr<Square> sq = core::checked_cast<Square>(q);
    core::Console::log(console, std::to_string(sq->get_area()));
    if (dynamic_cast<Rect const *>(s.get()) != nullptr) {
      core::Console::log(console, std::to_string(core::checked_cast<Rect>(s)->w));
    }
    std::type_index favorite = classes::same(std::type_index(typeid(int64_t)));
    static_cast<void>(favorite);
    int32_t named = 0;
    {
      int32_t i = 0;
      while (i < 6) {
        typeNamed(i);
        named = core::Int::add(named, 1);
        i = core::Int::add(i, 1);
      }
    }
    core::Console::log(console, std::to_string(named) + " types");
    std::shared_ptr<Sql> sql = Sql::make();
    sql->appendString(TEMPER_STR("it"));
    sql->appendString(nullptr);
    sql->appendInt(3);
    sql->appendShape(core::to_base<Shape>(Square::make(2)));
    sql->appendRect(Rect::make(1, 2));
    core::Console::log(console, sql->get_accumulated());
    int32_t seven = 7;
    std::shared_ptr<Sql> accumulator = Sql::make();
    {
      accumulator->appendSafe(TEMPER_STR("SELECT "));
      accumulator->appendInt(seven);
      accumulator->appendSafe(TEMPER_STR(", "));
      accumulator->appendString(TEMPER_STR("x"));
      accumulator->appendSafe(TEMPER_STR(" FROM "));
      accumulator->appendRect(Rect::make(2, 2));
    }
    core::Console::log(console, accumulator->get_accumulated());
  });
}

}  // namespace classes
