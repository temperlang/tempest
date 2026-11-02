// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace dense_bit_vector {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

}  // namespace dense_bit_vector

// init.cpp
#include <dense-bit-vector/init.hpp>

namespace dense_bit_vector {

namespace core = temper::core;

void global_init_init() {
  core::init_once([] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    std::shared_ptr<std::vector<bool>> v = core::DenseBitVector::make(16);
    core::Console::log(console, "v[2]=" + core::Boolean::toString(core::DenseBitVector::get(v, 2)));
    core::Console::log(console, "v[3]=" + core::Boolean::toString(core::DenseBitVector::get(v, 3)));
    core::Console::log(console, "v[3] <- true");
    core::DenseBitVector::set(v, 3, true);
    core::Console::log(console, "v[2]=" + core::Boolean::toString(core::DenseBitVector::get(v, 2)));
    core::Console::log(console, "v[3]=" + core::Boolean::toString(core::DenseBitVector::get(v, 3)));
    core::Console::log(console, "v[3] <- false");
    core::DenseBitVector::set(v, 3, false);
    core::Console::log(console, "v[2]=" + core::Boolean::toString(core::DenseBitVector::get(v, 2)));
    core::Console::log(console, "v[3]=" + core::Boolean::toString(core::DenseBitVector::get(v, 3)));
  });
}

}  // namespace dense_bit_vector
