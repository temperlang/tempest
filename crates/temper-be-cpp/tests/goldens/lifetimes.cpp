// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace lifetimes {

struct Box;
struct Holder;
struct Item;

struct Box : public std::enable_shared_from_this<Box> {
  std::shared_ptr<Item> item;
  static std::shared_ptr<Box> make(std::shared_ptr<Item> const & item);
  std::shared_ptr<Item> get_item();
};

struct Holder : public std::enable_shared_from_this<Holder> {
  std::shared_ptr<Box> box;
  static std::shared_ptr<Holder> make(std::shared_ptr<Box> const & box);
  std::string replace(std::shared_ptr<Box> const & old);
  std::string replaceItem(std::shared_ptr<Item> const & old);
  std::string run();
  std::string runItem();
  std::string drop();
  std::shared_ptr<Box> get_box();
};

struct Item : public std::enable_shared_from_this<Item> {
  std::string name;
  static std::shared_ptr<Item> make(std::string name);
  std::string leave(std::shared_ptr<Holder> const & h) const;
  std::string get_name() const;
};

// Runs the module's top level, once; call it before anything else.
void global_init_init();

}  // namespace lifetimes

// init.cpp
#include <lifetimes/init.hpp>

#include <mutex>

namespace lifetimes {

namespace core = temper::core;

std::shared_ptr<Box> Box::make(std::shared_ptr<Item> const & item) {
  std::shared_ptr<Box> self = std::make_shared<Box>();
  self->item = item;
  return self;
}

std::shared_ptr<Item> Box::get_item() {
  return this->item;
}

std::shared_ptr<Holder> Holder::make(std::shared_ptr<Box> const & box) {
  std::shared_ptr<Holder> self = std::make_shared<Holder>();
  self->box = box;
  return self;
}

std::string Holder::replace(std::shared_ptr<Box> const & old) {
  std::shared_ptr<Holder> self = shared_from_this();
  self->box = Box::make(Item::make("new"));
  return old->item->name;
}

std::string Holder::replaceItem(std::shared_ptr<Item> const & old) {
  std::shared_ptr<Holder> self = shared_from_this();
  self->box = Box::make(Item::make("newer"));
  return old->name;
}

std::string Holder::run() {
  std::shared_ptr<Holder> self = shared_from_this();
  std::shared_ptr<Box> value = self->box;
  return self->replace(value);
}

std::string Holder::runItem() {
  std::shared_ptr<Holder> self = shared_from_this();
  std::shared_ptr<Item> value = self->box->item;
  return self->replaceItem(value);
}

std::string Holder::drop() {
  std::shared_ptr<Holder> self = shared_from_this();
  return self->box->item->leave(self);
}

std::shared_ptr<Box> Holder::get_box() {
  return this->box;
}

std::shared_ptr<Item> Item::make(std::string name) {
  std::shared_ptr<Item> self = std::make_shared<Item>();
  self->name = name;
  return self;
}

std::string Item::leave(std::shared_ptr<Holder> const & h) const {
  std::shared_ptr<Item const> self = shared_from_this();
  h->box = Box::make(Item::make("other"));
  return self->name;
}

std::string Item::get_name() const {
  return this->name;
}

void global_init_init() {
  static std::once_flag once;
  std::call_once(once, [] {
    std::shared_ptr<core::Console::Type> console = core::Console::get_console();
    std::shared_ptr<Holder> h = Holder::make(Box::make(Item::make("old")));
    core::Console::log(console, h->run());
    core::Console::log(console, h->runItem());
    core::Console::log(console, h->box->item->name);
    core::Console::log(console, h->drop());
    core::Console::log(console, h->box->item->name);
    core::Console::log(console, Holder::make(Box::make(Item::make("chained")))->box->item->name);
  });
}

}  // namespace lifetimes
