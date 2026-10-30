// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace extra_class {

struct Stringable;
struct Circle;
struct Rectangle;

struct Stringable : public virtual temper::core::AnyValueBase {
  virtual std::string toString() const = 0;
};

struct Circle : public virtual Stringable {
  int32_t radius;
  static std::shared_ptr<Circle> make(int32_t radius);
  std::string toString() const override;
  int32_t get_radius() const;
};

struct Rectangle : public std::enable_shared_from_this<Rectangle> {
  int32_t width;
  int32_t height;
  static std::shared_ptr<Rectangle> make(int32_t width, int32_t height);
  int32_t get_width() const;
  int32_t get_height() const;
};

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace extra_class

// init.cpp
#include <extra-class/init.hpp>

#include <mutex>

namespace extra_class {

namespace core = temper::core;

std::shared_ptr<Circle> Circle::make(int32_t radius) {
  std::shared_ptr<Circle> self = std::make_shared<Circle>();
  self->radius = radius;
  return self;
}

std::string Circle::toString() const {
  return "Circle of radius " + std::to_string(this->radius);
}

int32_t Circle::get_radius() const {
  return this->radius;
}

std::shared_ptr<Rectangle> Rectangle::make(int32_t width, int32_t height) {
  std::shared_ptr<Rectangle> self = std::make_shared<Rectangle>();
  self->width = width;
  self->height = height;
  return self;
}

int32_t Rectangle::get_width() const {
  return this->width;
}

int32_t Rectangle::get_height() const {
  return this->height;
}

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {});
}

}  // namespace extra_class
