// init.hpp
#pragma once

#include <temper-core/core.hpp>
#include <std/net.hpp>

namespace netresponse {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

}  // namespace netresponse

// init.cpp
#include <netresponse/init.hpp>

namespace netresponse {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;
static core::GlobalSlot<std::string> url;

}  // namespace temper_globals

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::url.construct();
    temper_std::global_init_net();
    temper_globals::console.value = core::Console::get_console();
    temper_globals::url.value = "http://127.0.0.1:" + std::to_string(28461);
    auto fn = []() -> std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> {
      std::shared_ptr<int32_t> caseIndex = std::make_shared<int32_t>(0);
      std::shared_ptr<std::shared_ptr<temper_std::NetRequest>> req = std::make_shared<std::shared_ptr<temper_std::NetRequest>>(nullptr);
      std::shared_ptr<std::shared_ptr<temper_std::NetResponse>> resp = std::make_shared<std::shared_ptr<temper_std::NetResponse>>(nullptr);
      std::shared_ptr<std::string> body = std::make_shared<std::string>("");
      std::shared_ptr<core::NullableParam<std::string>> subject = std::make_shared<core::NullableParam<std::string>>(nullptr);
      std::shared_ptr<std::shared_ptr<core::Promise<std::shared_ptr<temper_std::NetResponse>>>> awaited = std::make_shared<std::shared_ptr<core::Promise<std::shared_ptr<temper_std::NetResponse>>>>(nullptr);
      std::shared_ptr<std::shared_ptr<core::Promise<core::NullableParam<std::string>>>> awaited2 = std::make_shared<std::shared_ptr<core::Promise<core::NullableParam<std::string>>>>(nullptr);
      auto convertedCoroutine = [=](std::shared_ptr<core::SafeGenerator<std::shared_ptr<core::Empty>>> generator) -> std::shared_ptr<core::GeneratorResult<std::shared_ptr<core::Empty>>> {
        while (true) {
          int32_t caseIndexLocal = (*caseIndex);
          *caseIndex = -1;
          switch (caseIndexLocal) {
            case 0:
              try {
                *req = temper_std::NetRequest::make(temper_globals::url.value);
                (*req)->post("[]", "application/json");
                *awaited = (*req)->send();
                *caseIndex = 2;
                core::awake_upon((*awaited), generator);
                return core::ValueResult<std::shared_ptr<core::Empty>>::make(core::empty());
              } catch (core::TemperBubble const &) {
                *caseIndex = 1;
              }
              break;
            case 1:
              core::Console::log(temper_globals::console.value, "failed");
              *caseIndex = -1;
              break;
            case 2:
              try {
                *resp = core::get_promise_result_sync((*awaited));
                if ((*resp)->get_status() == 200) {
                  *caseIndex = 3;
                } else {
                  *caseIndex = 5;
                }
              } catch (core::TemperBubble const &) {
                *caseIndex = 1;
              }
              break;
            case 3:
              try {
                *awaited2 = (*resp)->get_bodyContent();
                *caseIndex = 6;
                core::awake_upon((*awaited2), generator);
                return core::ValueResult<std::shared_ptr<core::Empty>>::make(core::empty());
              } catch (core::TemperBubble const &) {
                *caseIndex = 1;
              }
              break;
            case 4:
              try {
                *caseIndex = -1;
              } catch (core::TemperBubble const &) {
                *caseIndex = 1;
              }
              break;
            case 5:
              try {
                {
                  core::Console::log(temper_globals::console.value, "HTTP status was not 200: " + std::to_string((*resp)->get_status()) + "!");
                }
                *caseIndex = 4;
              } catch (core::TemperBubble const &) {
                *caseIndex = 1;
              }
              break;
            case 6:
              try {
                *subject = core::get_promise_result_sync((*awaited2));
                *body = (*subject) == nullptr ? "missing" : (*subject).value;
                core::Console::log(temper_globals::console.value, "Thanks, server.  I got " + (*body) + ".");
                *caseIndex = 4;
              } catch (core::TemperBubble const &) {
                *caseIndex = 1;
              }
              break;
            default:
              return core::doneResult();
          }
        }
      };
      return core::safe_adapt_generator_fn<std::shared_ptr<core::Empty>>(convertedCoroutine);
    };
    core::async_run(fn);
  });
}

}  // namespace netresponse
