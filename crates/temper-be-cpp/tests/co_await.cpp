// C++20 interop: a C++ coroutine `co_await`s Temper promises, as the
// runtime allows behind `__cpp_impl_coroutine`; `tests/functional.rs`
// builds it with `-std=c++20`.
#include <temper-core/core.hpp>
#include <coroutine>
#include <cstdio>
#include <exception>
#include <string>

namespace core = temper::core;

// A fire-and-forget coroutine, as a C++ user might write.
namespace {

struct Task {
    struct promise_type {
        Task get_return_object() { return {}; }
        std::suspend_never initial_suspend() noexcept { return {}; }
        std::suspend_never final_suspend() noexcept { return {}; }
        void return_void() {}
        void unhandled_exception() { std::terminate(); }
    };
};

Task use(std::shared_ptr<core::PromiseBuilder<std::string>> a,
         std::shared_ptr<core::PromiseBuilder<int32_t>> b) {
    std::string s = co_await core::getpromise(a);
    std::printf("got %s\n", s.c_str());
    try {
        int32_t n = co_await core::getpromise(b);
        std::printf("got %d\n", n);
    } catch (const core::TemperBubble&) {
        std::printf("broken\n");
    }
}

}  // namespace

int main() {
    auto a = core::PromiseBuilderNs::make<std::string>();
    auto b = core::PromiseBuilderNs::make<int32_t>();
    core::complete(a, "ready");  // settled before the await
    use(a, b);
    std::printf("waiting\n");
    core::breakpromise(b);       // resumes it
    std::printf("done\n");
}
