// init.hpp
#pragma once

#include <temper-core/core.hpp>
#include <std/testing.hpp>

namespace asserts {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

struct Vec2;
struct TestHelper;

struct Vec2 : public std::enable_shared_from_this<Vec2> {
  double x;
  double y;
  static std::shared_ptr<Vec2> make(double x, double y);
  std::string toString();
  double get_x() const;
  double get_y() const;
};

struct TestHelper : public std::enable_shared_from_this<TestHelper> {
  int32_t i;
  int32_t j;
  static std::shared_ptr<TestHelper> make(int32_t i, int32_t j);
  int32_t get_i() const;
  int32_t get_j() const;
};


#ifdef TEMPER_TESTS
// Tests, which `test.cpp` runs.
void aTestCase(std::shared_ptr<temper_std::Test> const & test);
void aFailingTest(std::shared_ptr<temper_std::Test> const & test);
void exported(std::shared_ptr<temper_std::Test> const & test);
#endif

}  // namespace asserts

// init.cpp
#include <asserts/init.hpp>

namespace asserts {

namespace core = temper::core;

static double normSquared(std::shared_ptr<Vec2> const & vec);

std::shared_ptr<Vec2> Vec2::make(double x, double y) {
  global_init_init();
  std::shared_ptr<Vec2> self = std::make_shared<Vec2>();
  self->x = x;
  self->y = y;
  return self;
}

std::string Vec2::toString() {
  std::shared_ptr<Vec2> self = shared_from_this();
  return "(" + core::Float64::toString(self->x) + ", " + core::Float64::toString(self->y) + ") size squared " + core::Float64::toString(asserts::normSquared(self));
}

double Vec2::get_x() const {
  return this->x;
}

double Vec2::get_y() const {
  return this->y;
}

static double normSquared(std::shared_ptr<Vec2> const & vec) {
  return vec->x * vec->x + vec->y * vec->y;
}

std::shared_ptr<TestHelper> TestHelper::make(int32_t i, int32_t j) {
  std::shared_ptr<TestHelper> self = std::make_shared<TestHelper>();
  self->i = i;
  self->j = j;
  return self;
}

int32_t TestHelper::get_i() const {
  return this->i;
}

int32_t TestHelper::get_j() const {
  return this->j;
}

static void assertMoreThings(std::shared_ptr<temper_std::Test> const & t, std::shared_ptr<TestHelper> const & helper, std::shared_ptr<std::vector<std::shared_ptr<std::vector<std::string>>>> const & strings) {
  {
    int32_t i = 0;
    while (i < core::List::length<std::shared_ptr<std::vector<std::string>>>(strings)) {
      {
        int32_t j = 0;
        while (j < core::List::length<std::string>(core::List::get<std::shared_ptr<std::vector<std::string>>>(strings, i))) {
          {
            int32_t actual = helper->i;
            int32_t expected = helper->j;
            t->assert_(actual == expected, [=]() -> std::string { return "expected helper.i == (" + std::to_string(expected) + ") not (" + std::to_string(actual) + ")"; });
          }
          std::string s = core::List::get<std::string>(core::List::get<std::shared_ptr<std::vector<std::string>>>(strings, i), j);
          {
            std::string actual = s;
            std::string expected = "hi";
            t->assert_(actual == expected, [=]() -> std::string { return "expected s == (" + expected + ") not (" + actual + ")"; });
          }
          j = core::Int::add(j, 1);
        }
      }
      i = core::Int::add(i, 1);
    }
  }
}

void global_init_init() {
  core::init_once([] {
    temper_std::global_init_testing();
    std::string name = "case";
  });
}

}  // namespace asserts

#ifdef TEMPER_TESTS

namespace asserts {

void aTestCase(std::shared_ptr<temper_std::Test> const & test) {
  test->assert_(core::Float64::eq(asserts::normSquared(Vec2::make(0.0, 0.0)), 0.0), []() -> std::string { return "Oh noes"; });
}

void aFailingTest(std::shared_ptr<temper_std::Test> const & test) {
  test->assert_(false, []() -> std::string { return "because it's wrong"; });
  std::shared_ptr<TestHelper> helper = TestHelper::make(3, 2);
  asserts::assertMoreThings(test, helper, core::List::make<std::shared_ptr<std::vector<std::string>>>(core::List::make<std::string>("bye")));
}

void exported(std::shared_ptr<temper_std::Test> const & test) {
  std::shared_ptr<Vec2> vec = Vec2::make(1.5, 2.5);
  {
    std::string actual = vec->toString();
    std::string expected = "(1.5, 2.5) size squared 8.5";
    test->assert_(actual == expected, [=]() -> std::string { return "expected vec.toString() == (" + expected + ") not (" + actual + ")"; });
  }
}

}  // namespace asserts

#endif  // TEMPER_TESTS

// main.cpp
#include "asserts/init.hpp"

int main() {
  asserts::global_init_init();
}

// test.cpp
#include "asserts/init.hpp"
#include "temper-core/test_main.hpp"

int main() {
  asserts::global_init_init();
  temper_std::global_init_testing();
  return temper::core::run_tests({
      {"a test case", [] {
        auto test = temper_std::Test::make();
        asserts::aTestCase(test);
        auto messages = test->messagesCombined();
        return temper::core::TestOutcome{
            test->get_passing(),
            temper::core::is_null(messages) ? std::string() : static_cast<std::string>(messages)};
      }},
      {"a failing test", [] {
        auto test = temper_std::Test::make();
        asserts::aFailingTest(test);
        auto messages = test->messagesCombined();
        return temper::core::TestOutcome{
            test->get_passing(),
            temper::core::is_null(messages) ? std::string() : static_cast<std::string>(messages)};
      }},
      {"exported", [] {
        auto test = temper_std::Test::make();
        asserts::exported(test);
        auto messages = test->messagesCombined();
        return temper::core::TestOutcome{
            test->get_passing(),
            temper::core::is_null(messages) ? std::string() : static_cast<std::string>(messages)};
      }},
  });
}

// CMakeLists.txt
# Generated by tempest; Kotlin Temper writes no build files.
cmake_minimum_required(VERSION 3.10)
project(asserts LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
find_package(Threads REQUIRED)
if(NOT TARGET std)
  add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../std ${CMAKE_CURRENT_BINARY_DIR}/std)
endif()

add_library(asserts init.cpp)
target_include_directories(asserts PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/..)
target_link_libraries(asserts PUBLIC Threads::Threads std)

add_executable(asserts-main main.cpp)
target_link_libraries(asserts-main PRIVATE asserts)

if(TRUE)
  add_executable(asserts-test test.cpp init.cpp)
  target_compile_definitions(asserts-test PRIVATE TEMPER_TESTS)
  target_include_directories(asserts-test PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/..)
  target_link_libraries(asserts-test PRIVATE Threads::Threads std)
endif()

