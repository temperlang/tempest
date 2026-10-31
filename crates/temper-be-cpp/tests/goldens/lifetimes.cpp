// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace lifetimes {

struct Box;
struct Holder;
struct Item;
struct Slot;

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

struct Slot : public std::enable_shared_from_this<Slot> {
  std::shared_ptr<Item> item;
  temper::core::NullableParam<std::string> tag;
  static std::shared_ptr<Slot> make(std::shared_ptr<Item> const & item, temper::core::NullableParam<std::string> tag);
  std::string swap(std::shared_ptr<Item> const & old);
  std::string run();
  std::string relabel();
  std::shared_ptr<Item> get_item();
  temper::core::NullableParam<std::string> get_tag();
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

std::shared_ptr<Slot> Slot::make(std::shared_ptr<Item> const & item, core::NullableParam<std::string> tag) {
  std::shared_ptr<Slot> self = std::make_shared<Slot>();
  self->item = item;
  self->tag = tag;
  return self;
}

std::string Slot::swap(std::shared_ptr<Item> const & old) {
  this->item = nullptr;
  this->tag = nullptr;
  if (old == nullptr) {
    return "none";
  } else {
    return old->name;
  }
}

std::string Slot::run() {
  std::shared_ptr<Slot> self = shared_from_this();
  std::shared_ptr<Item> value = self->item;
  return self->swap(value);
}

std::string Slot::relabel() {
  core::NullableParam<std::string> was = this->tag;
  this->tag = "after";
  {
    if (was == nullptr) {
      return "unlabeled";
    } else {
      return was.value;
    }
  }
}

std::shared_ptr<Item> Slot::get_item() {
  return this->item;
}

core::NullableParam<std::string> Slot::get_tag() {
  return this->tag;
}

static std::shared_ptr<Item> maybeItem(bool make) {
  if (make) {
    return Item::make("temp");
  } else {
    return nullptr;
  }
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
    std::shared_ptr<Slot> slot = Slot::make(Item::make("slotted"), "before");
    core::Console::log(console, slot->run());
    core::Console::log(console, slot->run());
    slot->tag = "first";
    core::Console::log(console, slot->relabel());
    core::Console::log(console, slot->relabel());
    std::shared_ptr<Item> subject = maybeItem(true);
    core::Console::log(console, (subject == nullptr ? Item::make("fallback") : subject)->name);
    std::shared_ptr<Item> subject2 = maybeItem(false);
    core::Console::log(console, (subject2 == nullptr ? Item::make("fallback") : subject2)->name);
    std::shared_ptr<Item> got = maybeItem(true);
    if (got != nullptr) {
      core::Console::log(console, got->name);
    }
  });
}

}  // namespace lifetimes
