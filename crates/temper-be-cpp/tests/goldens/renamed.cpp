// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace renamed {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

struct Checked;

struct Checked : public std::enable_shared_from_this<Checked> {
  int32_t end;
  static std::shared_ptr<Checked> make(int32_t end);
  int32_t delete_() const;
  int32_t get_local() const;
  int32_t get_end() const;
};

// Storage, which the library reads and its init sets; read values
// through the accessors below.
namespace temper_globals {
extern temper::core::GlobalSlot<int32_t> native;
}  // namespace temper_globals

int32_t const & native();

int32_t lambda(int32_t n);

}  // namespace renamed

// init.cpp
#include <renamed/init.hpp>

namespace renamed {

namespace core = temper::core;

namespace temper_globals {

core::GlobalSlot<int32_t> native;

}  // namespace temper_globals

int32_t const & native() {
  global_init_init();
  return temper_globals::native.value;
}

int32_t lambda(int32_t n) {
  global_init_init();
  return core::Int::add(n, 1);
}

std::shared_ptr<Checked> Checked::make(int32_t end) {
  global_init_init();
  std::shared_ptr<Checked> self = std::make_shared<Checked>();
  self->end = end;
  return self;
}

int32_t Checked::delete_() const {
  return core::Int::mul(this->end, 2);
}

int32_t Checked::get_local() const {
  return core::Int::add(this->end, temper_globals::native.value);
}

int32_t Checked::get_end() const {
  return this->end;
}

void global_init_init() {
  core::init_once([] {
    temper_globals::native.construct();
    temper_globals::native.value = 2;
  });
}

}  // namespace renamed
