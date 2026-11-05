// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace async {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

}  // namespace async

// init.cpp
#include <async/init.hpp>

namespace async {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;

}  // namespace temper_globals

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::console.value = core::Console::get_console();
    auto fn = []() -> std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> {
      std::shared_ptr<int32_t> caseIndex2 = std::make_shared<int32_t>(0);
      std::shared_ptr<std::shared_ptr<core::PromiseBuilder<std::string>>> b = std::make_shared<std::shared_ptr<core::PromiseBuilder<std::string>>>(nullptr);
      std::shared_ptr<std::shared_ptr<core::PromiseBuilder<std::string>>> b2 = std::make_shared<std::shared_ptr<core::PromiseBuilder<std::string>>>(nullptr);
      std::shared_ptr<std::shared_ptr<core::PromiseBuilder<std::string>>> b3 = std::make_shared<std::shared_ptr<core::PromiseBuilder<std::string>>>(nullptr);
      std::shared_ptr<std::shared_ptr<core::PromiseBuilder<std::string>>> b4 = std::make_shared<std::shared_ptr<core::PromiseBuilder<std::string>>>(nullptr);
      std::shared_ptr<std::shared_ptr<core::PromiseBuilder<std::shared_ptr<core::Empty>>>> a = std::make_shared<std::shared_ptr<core::PromiseBuilder<std::shared_ptr<core::Empty>>>>(nullptr);
      std::shared_ptr<std::shared_ptr<core::PromiseBuilder<std::shared_ptr<core::Empty>>>> b5 = std::make_shared<std::shared_ptr<core::PromiseBuilder<std::shared_ptr<core::Empty>>>>(nullptr);
      std::shared_ptr<std::shared_ptr<core::PromiseBuilder<std::string>>> b6 = std::make_shared<std::shared_ptr<core::PromiseBuilder<std::string>>>(nullptr);
      std::shared_ptr<std::string> t = std::make_shared<std::string>("");
      std::shared_ptr<std::shared_ptr<core::Promise<std::string>>> awaited2 = std::make_shared<std::shared_ptr<core::Promise<std::string>>>(nullptr);
      std::shared_ptr<std::string> t2 = std::make_shared<std::string>("");
      std::shared_ptr<std::shared_ptr<core::Promise<std::string>>> awaited3 = std::make_shared<std::shared_ptr<core::Promise<std::string>>>(nullptr);
      std::shared_ptr<std::string> t3 = std::make_shared<std::string>("");
      std::shared_ptr<std::shared_ptr<core::Promise<std::string>>> awaited4 = std::make_shared<std::shared_ptr<core::Promise<std::string>>>(nullptr);
      std::shared_ptr<std::string> t4 = std::make_shared<std::string>("");
      std::shared_ptr<std::shared_ptr<core::Promise<std::string>>> awaited5 = std::make_shared<std::shared_ptr<core::Promise<std::string>>>(nullptr);
      std::shared_ptr<std::string> t5 = std::make_shared<std::string>("");
      std::shared_ptr<std::shared_ptr<core::Promise<std::string>>> awaited6 = std::make_shared<std::shared_ptr<core::Promise<std::string>>>(nullptr);
      std::shared_ptr<std::shared_ptr<core::Promise<std::shared_ptr<core::Empty>>>> awaited7 = std::make_shared<std::shared_ptr<core::Promise<std::shared_ptr<core::Empty>>>>(nullptr);
      std::shared_ptr<std::string> t6 = std::make_shared<std::string>("");
      std::shared_ptr<std::shared_ptr<core::Promise<std::string>>> awaited8 = std::make_shared<std::shared_ptr<core::Promise<std::string>>>(nullptr);
      auto convertedCoroutine2 = [=](std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> generator2) -> std::shared_ptr<core::GeneratorResult<std::shared_ptr<core::Empty>>> {
        while (true) {
          int32_t caseIndexLocal2 = (*caseIndex2);
          if (caseIndexLocal2 == 0) {
            try {
              *b = core::PromiseBuilderNs::make<std::string>();
              core::complete((*b), std::string("Completed synchronously"));
              *awaited2 = core::getpromise((*b));
              *caseIndex2 = 3;
              core::awake_upon((*awaited2), generator2);
              return core::ValueResult<std::shared_ptr<core::Empty>>::make(core::empty());
            } catch (core::TemperBubble const &) {
              *caseIndex2 = 1;
            }
          } else if (caseIndexLocal2 == 1) {
            core::panic();
          } else if (caseIndexLocal2 == 2) {
            *b2 = core::PromiseBuilderNs::make<std::string>();
            core::breakpromise((*b2));
            *caseIndex2 = 6;
          } else if (caseIndexLocal2 == 3) {
            try {
              *t = core::get_promise_result_sync((*awaited2));
              core::Console::log(temper_globals::console.value, (*t));
              *caseIndex2 = 2;
            } catch (core::TemperBubble const &) {
              *caseIndex2 = 1;
            }
          } else if (caseIndexLocal2 == 4) {
            *t2 = "Broken synchronously";
            *caseIndex2 = 5;
          } else if (caseIndexLocal2 == 5) {
            core::Console::log(temper_globals::console.value, (*t2));
            *caseIndex2 = 10;
          } else if (caseIndexLocal2 == 6) {
            try {
              *awaited3 = core::getpromise((*b2));
              *caseIndex2 = 7;
              core::awake_upon((*awaited3), generator2);
              return core::ValueResult<std::shared_ptr<core::Empty>>::make(core::empty());
            } catch (core::TemperBubble const &) {
              *caseIndex2 = 4;
            }
          } else if (caseIndexLocal2 == 7) {
            try {
              *t2 = core::get_promise_result_sync((*awaited3));
              *caseIndex2 = 5;
            } catch (core::TemperBubble const &) {
              *caseIndex2 = 4;
            }
          } else if (caseIndexLocal2 == 8) {
            core::panic();
          } else if (caseIndexLocal2 == 9) {
            try {
              *b4 = core::PromiseBuilderNs::make<std::string>();
              core::complete((*b4), std::string("Worth doing twice"));
              *awaited5 = core::getpromise((*b4));
              *caseIndex2 = 14;
              core::awake_upon((*awaited5), generator2);
              return core::ValueResult<std::shared_ptr<core::Empty>>::make(core::empty());
            } catch (core::TemperBubble const &) {
              *caseIndex2 = 12;
            }
          } else if (caseIndexLocal2 == 10) {
            try {
              *b3 = core::PromiseBuilderNs::make<std::string>();
              auto fn2 = [=]() -> std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> {
                std::shared_ptr<int32_t> caseIndex = std::make_shared<int32_t>(0);
                auto convertedCoroutine = [=](std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>>) -> std::shared_ptr<core::GeneratorResult<std::shared_ptr<core::Empty>>> {
                  int32_t caseIndexLocal = (*caseIndex);
                  if (caseIndexLocal == 0) {
                    core::complete((*b3), std::string("Completed asynchronously"));
                    *caseIndex = -1;
                    return core::doneResult();
                  } else {
                    return core::doneResult();
                  }
                };
                return core::safe_adapt_generator_fn<std::shared_ptr<core::Empty>>(convertedCoroutine);
              };
              core::async_run(fn2);
              *awaited4 = core::getpromise((*b3));
              *caseIndex2 = 11;
              core::awake_upon((*awaited4), generator2);
              return core::ValueResult<std::shared_ptr<core::Empty>>::make(core::empty());
            } catch (core::TemperBubble const &) {
              *caseIndex2 = 8;
            }
          } else if (caseIndexLocal2 == 11) {
            try {
              *t3 = core::get_promise_result_sync((*awaited4));
              core::Console::log(temper_globals::console.value, (*t3));
              *caseIndex2 = 9;
            } catch (core::TemperBubble const &) {
              *caseIndex2 = 8;
            }
          } else if (caseIndexLocal2 == 12) {
            core::panic();
          } else if (caseIndexLocal2 == 13) {
            try {
              *a = core::PromiseBuilderNs::make<std::shared_ptr<core::Empty>>();
              *b5 = core::PromiseBuilderNs::make<std::shared_ptr<core::Empty>>();
              auto fn2 = [=]() -> std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> {
                std::shared_ptr<int32_t> caseIndex = std::make_shared<int32_t>(0);
                std::shared_ptr<std::shared_ptr<core::Promise<std::shared_ptr<core::Empty>>>> awaited = std::make_shared<std::shared_ptr<core::Promise<std::shared_ptr<core::Empty>>>>(nullptr);
                auto convertedCoroutine = [=](std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> generator) -> std::shared_ptr<core::GeneratorResult<std::shared_ptr<core::Empty>>> {
                  while (true) {
                    int32_t caseIndexLocal = (*caseIndex);
                    if (caseIndexLocal == 0) {
                      try {
                        *awaited = core::getpromise((*b5));
                        *caseIndex = 3;
                        core::awake_upon((*awaited), generator);
                        return core::ValueResult<std::shared_ptr<core::Empty>>::make(core::empty());
                      } catch (core::TemperBubble const &) {
                        *caseIndex = 1;
                      }
                    } else if (caseIndexLocal == 1) {
                      core::panic();
                    } else if (caseIndexLocal == 2) {
                      core::Console::log(temper_globals::console.value, "After");
                      core::complete((*a), core::empty());
                      *caseIndex = -1;
                      return core::doneResult();
                    } else if (caseIndexLocal == 3) {
                      try {
                        core::get_promise_result_sync((*awaited));
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
              auto fn3 = [=]() -> std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> {
                std::shared_ptr<int32_t> caseIndex = std::make_shared<int32_t>(0);
                auto convertedCoroutine = [=](std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>>) -> std::shared_ptr<core::GeneratorResult<std::shared_ptr<core::Empty>>> {
                  int32_t caseIndexLocal = (*caseIndex);
                  if (caseIndexLocal == 0) {
                    core::Console::log(temper_globals::console.value, "Before");
                    core::complete((*b5), core::empty());
                    *caseIndex = -1;
                    return core::doneResult();
                  } else {
                    return core::doneResult();
                  }
                };
                return core::safe_adapt_generator_fn<std::shared_ptr<core::Empty>>(convertedCoroutine);
              };
              core::async_run(fn3);
              *awaited7 = core::getpromise((*a));
              *caseIndex2 = 18;
              core::awake_upon((*awaited7), generator2);
              return core::ValueResult<std::shared_ptr<core::Empty>>::make(core::empty());
            } catch (core::TemperBubble const &) {
              *caseIndex2 = 16;
            }
          } else if (caseIndexLocal2 == 14) {
            try {
              *t4 = core::get_promise_result_sync((*awaited5));
              core::Console::log(temper_globals::console.value, (*t4));
              *awaited6 = core::getpromise((*b4));
              *caseIndex2 = 15;
              core::awake_upon((*awaited6), generator2);
              return core::ValueResult<std::shared_ptr<core::Empty>>::make(core::empty());
            } catch (core::TemperBubble const &) {
              *caseIndex2 = 12;
            }
          } else if (caseIndexLocal2 == 15) {
            try {
              *t5 = core::get_promise_result_sync((*awaited6));
              core::Console::log(temper_globals::console.value, (*t5));
              *caseIndex2 = 13;
            } catch (core::TemperBubble const &) {
              *caseIndex2 = 12;
            }
          } else if (caseIndexLocal2 == 16) {
            core::panic();
          } else if (caseIndexLocal2 == 17) {
            try {
              core::Console::log(temper_globals::console.value, "What is your favourite colour?");
              *b6 = core::PromiseBuilderNs::make<std::string>();
              core::complete((*b6), std::string("Blue"));
              core::complete((*b6), std::string("No, yell..."));
              *awaited8 = core::getpromise((*b6));
              *caseIndex2 = 21;
              core::awake_upon((*awaited8), generator2);
              return core::ValueResult<std::shared_ptr<core::Empty>>::make(core::empty());
            } catch (core::TemperBubble const &) {
              *caseIndex2 = 19;
            }
          } else if (caseIndexLocal2 == 18) {
            try {
              core::get_promise_result_sync((*awaited7));
              *caseIndex2 = 17;
            } catch (core::TemperBubble const &) {
              *caseIndex2 = 16;
            }
          } else if (caseIndexLocal2 == 19) {
            *caseIndex2 = 20;
          } else if (caseIndexLocal2 == 20) {
            *caseIndex2 = -1;
            return core::doneResult();
          } else if (caseIndexLocal2 == 21) {
            try {
              *t6 = core::get_promise_result_sync((*awaited8));
              core::Console::log(temper_globals::console.value, (*t6));
              *caseIndex2 = 20;
            } catch (core::TemperBubble const &) {
              *caseIndex2 = 19;
            }
          } else {
            return core::doneResult();
          }
        }
      };
      return core::safe_adapt_generator_fn<std::shared_ptr<core::Empty>>(convertedCoroutine2);
    };
    core::async_run(fn);
  });
}

}  // namespace async
