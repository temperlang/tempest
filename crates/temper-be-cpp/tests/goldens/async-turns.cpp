// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace async_turns {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

struct Tick;
struct Counter;

struct Tick : public virtual temper::core::AnyValueBase {
  virtual void apply(int32_t i) const = 0;
};

struct Counter : public std::enable_shared_from_this<Counter> {
  int32_t n;
  static std::shared_ptr<Counter> make();
  void bump(std::string tag, int32_t times);
  int32_t get_n();
};

std::shared_ptr<temper::core::Promise<int32_t>> later(int32_t n);
std::shared_ptr<temper::core::SafeGenerator<std::shared_ptr<temper::core::Empty>>> ticks(int32_t n, std::function<void(int32_t)> const & each);

}  // namespace async_turns

// init.cpp
#include <async-turns/init.hpp>

namespace async_turns {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;
static core::GlobalSlot<std::shared_ptr<Counter>> c;
static core::GlobalSlot<std::shared_ptr<core::PromiseBuilder<int32_t>>> gateA;
static core::GlobalSlot<std::shared_ptr<core::PromiseBuilder<int32_t>>> gateB;

}  // namespace temper_globals

std::shared_ptr<core::Promise<int32_t>> later(int32_t n) {
  global_init_init();
  std::shared_ptr<core::PromiseBuilder<int32_t>> pb = core::PromiseBuilderNs::make<int32_t>();
  auto fn = [=]() -> std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> {
    std::shared_ptr<int32_t> caseIndex = std::make_shared<int32_t>(0);
    auto convertedCoroutine = [=](std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>>) -> std::shared_ptr<core::GeneratorResult<std::shared_ptr<core::Empty>>> {
      int32_t caseIndexLocal = (*caseIndex);
      *caseIndex = -1;
      if (caseIndexLocal == 0) {
        core::complete(pb, core::Int::mul(n, 2));
        return core::doneResult();
      } else {
        return core::doneResult();
      }
    };
    return core::safe_adapt_generator_fn<std::shared_ptr<core::Empty>>(convertedCoroutine);
  };
  core::async_run(fn);
  return core::getpromise(pb);
}

static std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> generated(std::function<std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>>()> const & body) {
  return body();
}

std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> ticks(int32_t n, std::function<void(int32_t)> const & each) {
  global_init_init();
  auto fn = [=]() -> std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> {
    std::shared_ptr<int32_t> caseIndex = std::make_shared<int32_t>(0);
    std::shared_ptr<int32_t> i = std::make_shared<int32_t>(0);
    auto convertedCoroutine = [=](std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>>) -> std::shared_ptr<core::GeneratorResult<std::shared_ptr<core::Empty>>> {
      while (true) {
        int32_t caseIndexLocal = (*caseIndex);
        *caseIndex = -1;
        if (caseIndexLocal == 0) {
          *i = 0;
          *caseIndex = 1;
        } else if (caseIndexLocal == 1) {
          if ((*i) < n) {
            *caseIndex = 2;
          } else {
            *caseIndex = 3;
          }
        } else if (caseIndexLocal == 2) {
          each((*i));
          *caseIndex = 4;
          return core::ValueResult<std::shared_ptr<core::Empty>>::make(core::empty());
        } else if (caseIndexLocal == 4) {
          *i = core::Int::add((*i), 1);
          *caseIndex = 1;
        } else {
          return core::doneResult();
        }
      }
    };
    return core::safe_adapt_generator_fn<std::shared_ptr<core::Empty>>(convertedCoroutine);
  };
  return async_turns::generated(fn);
}

std::shared_ptr<Counter> Counter::make() {
  std::shared_ptr<Counter> self = std::make_shared<Counter>();
  self->n = 0;
  return self;
}

void Counter::bump(std::string tag, int32_t times) {
  core::Console::log(temper_globals::console.value, tag + " start");
  {
    int32_t i = 0;
    while (i < times) {
      int32_t before = this->n;
      this->n = core::Int::add(before, 1);
      i = core::Int::add(i, 1);
    }
  }
  core::Console::log(temper_globals::console.value, tag + " done n=" + std::to_string(this->n));
}

int32_t Counter::get_n() {
  return this->n;
}

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::c.construct();
    temper_globals::gateA.construct();
    temper_globals::gateB.construct();
    temper_globals::console.value = core::Console::get_console();
    std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> t = async_turns::ticks(2, [](int32_t i) -> void { core::Console::log(temper_globals::console.value, "tick " + std::to_string(i)); });
    core::next(t);
    core::next(t);
    core::next(t);
    temper_globals::c.value = Counter::make();
    temper_globals::gateA.value = core::PromiseBuilderNs::make<int32_t>();
    temper_globals::gateB.value = core::PromiseBuilderNs::make<int32_t>();
    auto fn2 = []() -> std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> {
      std::shared_ptr<int32_t> caseIndex = std::make_shared<int32_t>(0);
      std::shared_ptr<int32_t> k = std::make_shared<int32_t>(0);
      std::shared_ptr<std::shared_ptr<core::Promise<int32_t>>> awaited = std::make_shared<std::shared_ptr<core::Promise<int32_t>>>(nullptr);
      auto convertedCoroutine = [=](std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> generator) -> std::shared_ptr<core::GeneratorResult<std::shared_ptr<core::Empty>>> {
        while (true) {
          int32_t caseIndexLocal = (*caseIndex);
          *caseIndex = -1;
          if (caseIndexLocal == 0) {
            temper_globals::c.value->bump("A", 10000);
            *caseIndex = 3;
          } else if (caseIndexLocal == 1) {
            *k = -1;
            *caseIndex = 2;
          } else if (caseIndexLocal == 2) {
            temper_globals::c.value->bump("A" + std::to_string((*k)), 10000);
            return core::doneResult();
          } else if (caseIndexLocal == 3) {
            try {
              *awaited = core::getpromise(temper_globals::gateA.value);
              *caseIndex = 4;
              core::awake_upon((*awaited), generator);
              return core::ValueResult<std::shared_ptr<core::Empty>>::make(core::empty());
            } catch (core::TemperBubble const &) {
              *caseIndex = 1;
            }
          } else if (caseIndexLocal == 4) {
            try {
              *k = core::get_promise_result_sync((*awaited));
              *caseIndex = 2;
            } catch (core::TemperBubble const &) {
              *caseIndex = 1;
            }
          } else {
            return core::doneResult();
          }
        }
      };
      return core::safe_adapt_generator_fn<std::shared_ptr<core::Empty>>(convertedCoroutine);
    };
    core::async_run(fn2);
    auto fn3 = []() -> std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> {
      std::shared_ptr<int32_t> caseIndex = std::make_shared<int32_t>(0);
      std::shared_ptr<int32_t> k = std::make_shared<int32_t>(0);
      std::shared_ptr<std::shared_ptr<core::Promise<int32_t>>> awaited = std::make_shared<std::shared_ptr<core::Promise<int32_t>>>(nullptr);
      auto convertedCoroutine = [=](std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> generator) -> std::shared_ptr<core::GeneratorResult<std::shared_ptr<core::Empty>>> {
        while (true) {
          int32_t caseIndexLocal = (*caseIndex);
          *caseIndex = -1;
          if (caseIndexLocal == 0) {
            temper_globals::c.value->bump("B", 10000);
            *caseIndex = 3;
          } else if (caseIndexLocal == 1) {
            *k = -1;
            *caseIndex = 2;
          } else if (caseIndexLocal == 2) {
            temper_globals::c.value->bump("B" + std::to_string((*k)), 10000);
            return core::doneResult();
          } else if (caseIndexLocal == 3) {
            try {
              *awaited = core::getpromise(temper_globals::gateB.value);
              *caseIndex = 4;
              core::awake_upon((*awaited), generator);
              return core::ValueResult<std::shared_ptr<core::Empty>>::make(core::empty());
            } catch (core::TemperBubble const &) {
              *caseIndex = 1;
            }
          } else if (caseIndexLocal == 4) {
            try {
              *k = core::get_promise_result_sync((*awaited));
              *caseIndex = 2;
            } catch (core::TemperBubble const &) {
              *caseIndex = 1;
            }
          } else {
            return core::doneResult();
          }
        }
      };
      return core::safe_adapt_generator_fn<std::shared_ptr<core::Empty>>(convertedCoroutine);
    };
    core::async_run(fn3);
    auto fn4 = []() -> std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> {
      std::shared_ptr<int32_t> caseIndex = std::make_shared<int32_t>(0);
      auto convertedCoroutine = [=](std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>>) -> std::shared_ptr<core::GeneratorResult<std::shared_ptr<core::Empty>>> {
        int32_t caseIndexLocal = (*caseIndex);
        *caseIndex = -1;
        if (caseIndexLocal == 0) {
          core::Console::log(temper_globals::console.value, "opener: calling complete");
          core::complete(temper_globals::gateA.value, 1);
          core::complete(temper_globals::gateB.value, 2);
          core::Console::log(temper_globals::console.value, "opener: complete returned");
          return core::doneResult();
        } else {
          return core::doneResult();
        }
      };
      return core::safe_adapt_generator_fn<std::shared_ptr<core::Empty>>(convertedCoroutine);
    };
    core::async_run(fn4);
  });
}

}  // namespace async_turns
