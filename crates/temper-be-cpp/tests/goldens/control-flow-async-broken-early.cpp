// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace async_broken_early {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

}  // namespace async_broken_early

// init.cpp
#include <async-broken-early/init.hpp>

namespace async_broken_early {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;
static core::GlobalSlot<std::shared_ptr<core::PromiseBuilder<int32_t>>> b;

}  // namespace temper_globals

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::b.construct();
    temper_globals::console.value = core::Console::get_console();
    temper_globals::b.value = core::PromiseBuilderNs::make<int32_t>();
    auto fn = []() -> std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> {
      std::shared_ptr<int32_t> caseIndex = std::make_shared<int32_t>(0);
      std::shared_ptr<int32_t> v = std::make_shared<int32_t>(0);
      std::shared_ptr<std::shared_ptr<core::Promise<int32_t>>> awaited = std::make_shared<std::shared_ptr<core::Promise<int32_t>>>(nullptr);
      auto convertedCoroutine = [=](std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> generator) -> std::shared_ptr<core::GeneratorResult<std::shared_ptr<core::Empty>>> {
        while (true) {
          int32_t caseIndexLocal = (*caseIndex);
          if (caseIndexLocal == 0) {
            try {
              *awaited = core::getpromise(temper_globals::b.value);
              *caseIndex = 3;
              core::awake_upon((*awaited), generator);
              return core::ValueResult<std::shared_ptr<core::Empty>>::make(core::empty());
            } catch (core::TemperBubble const &) {
              *caseIndex = 1;
            }
          } else if (caseIndexLocal == 1) {
            *v = -1;
            *caseIndex = 2;
          } else if (caseIndexLocal == 2) {
            core::Console::log(temper_globals::console.value, "got " + std::to_string((*v)));
            *caseIndex = -1;
            return core::doneResult();
          } else if (caseIndexLocal == 3) {
            try {
              *v = core::get_promise_result_sync((*awaited));
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
    core::async_run(fn);
    core::breakpromise(temper_globals::b.value);
  });
}

}  // namespace async_broken_early
