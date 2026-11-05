// Host code plugging its own client into std/net, for `functional.rs`'s
// `net_client_from_host`: before the generated `netresponse` and
// `netresponse-not-found` libraries run, it sets a client that answers as
// the test server does, from threads of its own, so each response reaches
// Temper code through the runtime's inbox. They log as the funtests
// expect, then this prints how many requests the client saw.
#include "netresponse/init.hpp"
#include "netresponse-not-found/init.hpp"
#include <temper-core/net.hpp>
#include <chrono>
#include <cstdio>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <utility>
#include <vector>

namespace net = temper::core::net;

namespace {

// Answers as the test server: 404 at `/missing`, nothing at port 1, and
// otherwise what the method was and whether the body was JSON.
class FakeServer final : public net::Client {
  public:
    void send(net::Request request, net::Respond respond) override {
        std::lock_guard<std::mutex> lock(mutex_);
        threads_.emplace_back([request, respond]() {
            std::this_thread::sleep_for(std::chrono::milliseconds(5));
            if (request.url.find(":1/") != std::string::npos) {
                respond(nullptr);
                return;
            }
            auto response = std::make_shared<net::Response>();
            if (request.url.find("/missing") != std::string::npos) {
                response->status = 404;
                response->contentType = std::string("text/plain");
                response->bodyContent = std::string("not here");
            } else {
                bool json = request.bodyMimeType.has_value && request.bodyMimeType.value == "application/json";
                response->status = 200;
                response->contentType = std::string("application/json");
                response->bodyContent = "{\"method\": \"" + request.method + "\", \"json\": " + (json ? "true" : "false") + "}";
            }
            respond(response);
            // A second answer is ignored.
            respond(nullptr);
        });
    }

    std::size_t join() {
        std::lock_guard<std::mutex> lock(mutex_);
        for (std::thread& thread : threads_) {
            thread.join();
        }
        return threads_.size();
    }

  private:
    std::mutex mutex_;
    std::vector<std::thread> threads_;
};

}  // namespace

int main() {
    auto server = std::make_shared<FakeServer>();
    net::set_client(server);
    netresponse::global_init_init();
    netresponse_not_found::global_init_init();
    std::printf("requests %zu\n", server->join());
    net::set_client(nullptr);
}
