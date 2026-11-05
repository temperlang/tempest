// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace await_in_loop {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

}  // namespace await_in_loop

// init.cpp
#include <await-in-loop/init.hpp>

namespace await_in_loop {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;
static core::GlobalSlot<std::shared_ptr<core::PromiseBuilder<std::string>>> once;
static core::GlobalSlot<std::shared_ptr<core::PromiseBuilder<std::string>>> later;

}  // namespace temper_globals

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::once.construct();
    temper_globals::later.construct();
    temper_globals::console.value = core::Console::get_console();
    temper_globals::once.value = core::PromiseBuilderNs::make<std::string>();
    core::complete(temper_globals::once.value, std::string("same"));
    temper_globals::later.value = core::PromiseBuilderNs::make<std::string>();
    auto fn = []() -> std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> {
      std::shared_ptr<int32_t> caseIndex = std::make_shared<int32_t>(0);
      std::shared_ptr<int32_t> i = std::make_shared<int32_t>(0);
      std::shared_ptr<std::string> s = std::make_shared<std::string>("");
      std::shared_ptr<int32_t> j = std::make_shared<int32_t>(0);
      std::shared_ptr<std::string> s2 = std::make_shared<std::string>("");
      std::shared_ptr<std::string> t = std::make_shared<std::string>("");
      std::shared_ptr<std::shared_ptr<core::Promise<std::string>>> awaited = std::make_shared<std::shared_ptr<core::Promise<std::string>>>(nullptr);
      std::shared_ptr<std::shared_ptr<core::Promise<std::string>>> awaited2 = std::make_shared<std::shared_ptr<core::Promise<std::string>>>(nullptr);
      std::shared_ptr<std::shared_ptr<core::Promise<std::string>>> awaited3 = std::make_shared<std::shared_ptr<core::Promise<std::string>>>(nullptr);
      auto convertedCoroutine = [=](std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> generator) -> std::shared_ptr<core::GeneratorResult<std::shared_ptr<core::Empty>>> {
        while (true) {
          int32_t caseIndexLocal = (*caseIndex);
          *caseIndex = -1;
          if (caseIndexLocal == 0) {
            *i = 0;
            *caseIndex = 1;
          } else if (caseIndexLocal == 1) {
            if ((*i) < 2) {
              *caseIndex = 2;
            } else {
              *caseIndex = 3;
            }
          } else if (caseIndexLocal == 2) {
            try {
              *awaited = core::getpromise(temper_globals::once.value);
              *caseIndex = 6;
              core::awake_upon((*awaited), generator);
              return core::ValueResult<std::shared_ptr<core::Empty>>::make(core::empty());
            } catch (core::TemperBubble const &) {
              *caseIndex = 4;
            }
          } else if (caseIndexLocal == 3) {
            *j = 0;
            *caseIndex = 7;
          } else if (caseIndexLocal == 4) {
            *s = "?";
            *caseIndex = 5;
          } else if (caseIndexLocal == 5) {
            core::Console::log(temper_globals::console.value, (*s) + " " + std::to_string((*i)));
            *i = core::Int::add((*i), 1);
            *caseIndex = 1;
          } else if (caseIndexLocal == 6) {
            try {
              *s = core::get_promise_result_sync((*awaited));
              *caseIndex = 5;
            } catch (core::TemperBubble const &) {
              *caseIndex = 4;
            }
          } else if (caseIndexLocal == 7) {
            if ((*j) < 2) {
              *caseIndex = 8;
            } else {
              *caseIndex = 9;
            }
          } else if (caseIndexLocal == 8) {
            try {
              *awaited2 = core::getpromise(temper_globals::later.value);
              *caseIndex = 12;
              core::awake_upon((*awaited2), generator);
              return core::ValueResult<std::shared_ptr<core::Empty>>::make(core::empty());
            } catch (core::TemperBubble const &) {
              *caseIndex = 10;
            }
          } else if (caseIndexLocal == 9) {
            try {
              *awaited3 = core::getpromise(temper_globals::once.value);
              *caseIndex = 15;
              core::awake_upon((*awaited3), generator);
              return core::ValueResult<std::shared_ptr<core::Empty>>::make(core::empty());
            } catch (core::TemperBubble const &) {
              *caseIndex = 13;
            }
          } else if (caseIndexLocal == 10) {
            *s2 = "?";
            *caseIndex = 11;
          } else if (caseIndexLocal == 11) {
            core::Console::log(temper_globals::console.value, (*s2) + " " + std::to_string((*j)));
            *j = core::Int::add((*j), 1);
            *caseIndex = 7;
          } else if (caseIndexLocal == 12) {
            try {
              *s2 = core::get_promise_result_sync((*awaited2));
              *caseIndex = 11;
            } catch (core::TemperBubble const &) {
              *caseIndex = 10;
            }
          } else if (caseIndexLocal == 13) {
            *t = "?";
            *caseIndex = 14;
          } else if (caseIndexLocal == 14) {
            core::Console::log(temper_globals::console.value, (*t) + " after the loops");
            return core::doneResult();
          } else if (caseIndexLocal == 15) {
            try {
              *t = core::get_promise_result_sync((*awaited3));
              *caseIndex = 14;
            } catch (core::TemperBubble const &) {
              *caseIndex = 13;
            }
          } else {
            return core::doneResult();
          }
        }
      };
      return core::safe_adapt_generator_fn<std::shared_ptr<core::Empty>>(convertedCoroutine);
    };
    core::async_run(fn);
    core::complete(temper_globals::later.value, std::string("later"));
  });
}

}  // namespace await_in_loop
