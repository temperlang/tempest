// init.hpp
#pragma once

#include <temper-core/core.hpp>

namespace map {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

}  // namespace map

// init.cpp
#include <map/init.hpp>

namespace map {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;
static core::GlobalSlot<std::shared_ptr<core::Mapped::Ordered<int32_t, std::string>>> messages;
static core::GlobalSlot<std::shared_ptr<core::Mapped::Ordered<std::string, std::shared_ptr<std::vector<bool>>>>> eightColors;

}  // namespace temper_globals

template <class K, class V>
static std::shared_ptr<std::vector<K>> listKeys(std::shared_ptr<core::Mapped::Ordered<K, V>> const & mapped) {
  return core::List::map(core::Mapped::toList<K, V>(mapped), [](std::shared_ptr<core::Pair<K, V>> entry) -> K { return entry->key; });
}

static std::string messageDefined(int32_t code) {
  return core::Boolean::toString(core::Mapped::has<int32_t, std::string>(temper_globals::messages.value, code));
}

static std::string colorToHex(std::shared_ptr<std::vector<bool>> const & color) {
  std::string c = core::List::join(color, "", [](bool b) -> std::string { if (b) { return "FF"; } else { return "00"; } });
  return "#" + c;
}

static void printMap(std::string name, std::shared_ptr<core::Mapped::Ordered<std::string, std::string>> const & map) {
  core::Console::log(temper_globals::console.value, "--- " + name + " ---");
  core::Mapped::forEach(map, [](std::string key, std::string value) -> void { core::Console::log(temper_globals::console.value, key + " = " + value); });
}

static void printMapBuilder(std::string name, std::shared_ptr<core::Mapped::Ordered<std::string, std::string>> const & map) {
  core::Console::log(temper_globals::console.value, "--- " + name + " ---");
  core::Mapped::forEach(map, [](std::string key, std::string value) -> void { core::Console::log(temper_globals::console.value, key + " = " + value); });
}

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::messages.construct();
    temper_globals::eightColors.construct();
    temper_globals::console.value = core::Console::get_console();
    std::shared_ptr<core::Pair<int32_t, std::string>> pair = core::PairFactory::make<int32_t, std::string>(200, "OK");
    std::shared_ptr<core::Pair<int32_t, std::string>> value2 = core::PairFactory::make<int32_t, std::string>(404, "Not Found");
    temper_globals::messages.value = core::Map::make<int32_t, std::string>(core::List::make<std::shared_ptr<core::Pair<int32_t, std::string>>>(pair, value2, core::PairFactory::make<int32_t, std::string>(500, "Internal Server Error")));
    std::string message = core::Mapped::get<int32_t, std::string>(temper_globals::messages.value, 200);
    core::Console::log(temper_globals::console.value, "Message: " + message);
    std::string string = messageDefined(200);
    core::Console::log(temper_globals::console.value, "Has 200: " + string + ", 302: " + messageDefined(302));
    std::string value3;
    try {
      value3 = core::Mapped::get<int32_t, std::string>(temper_globals::messages.value, 302);
    } catch (core::TemperBubble const &) {
      value3 = "Found";
    }
    core::Console::log(temper_globals::console.value, "Fallback: " + value3);
    core::Console::log(temper_globals::console.value, "Codes: " + core::List::join(map::listKeys<int32_t, std::string>(temper_globals::messages.value), " ", [](int32_t it) -> std::string { return std::to_string(it); }));
    core::Console::log(temper_globals::console.value, "Again: " + core::List::join(core::Mapped::keys<int32_t, std::string>(temper_globals::messages.value), " ", [](int32_t it) -> std::string { return std::to_string(it); }));
    core::Console::log(temper_globals::console.value, "Messages: " + core::List::join(core::Mapped::values<int32_t, std::string>(temper_globals::messages.value), ", ", [](std::string it) -> std::string { return it; }));
    std::shared_ptr<std::vector<int32_t>> groups = core::Mapped::toListWith(temper_globals::messages.value, [](int32_t key, std::string) -> int32_t { return key / 100; });
    core::Console::log(temper_globals::console.value, "Groups: " + core::List::join(groups, " ", [](int32_t it) -> std::string { return std::to_string(it); }));
    std::shared_ptr<core::Mapped::Ordered<std::string, std::shared_ptr<std::vector<std::string>>>> builder2 = core::Map::make<std::string, std::shared_ptr<std::vector<std::string>>>();
    core::MapBuilder::set<std::string, std::shared_ptr<std::vector<std::string>>>(builder2, "Honduras", core::List::make<std::string>("El Salvador", "Guatemala", "Nicaragua"));
    core::Console::log(temper_globals::console.value, "From builder: " + core::List::join(core::Mapped::get<std::string, std::shared_ptr<std::vector<std::string>>>(builder2, "Honduras"), ", ", [](std::string it) -> std::string { return it; }));
    core::Console::log(temper_globals::console.value, "Key so far: " + core::List::get<std::string>(map::listKeys<std::string, std::shared_ptr<std::vector<std::string>>>(builder2), 0));
    core::MapBuilder::set<std::string, std::shared_ptr<std::vector<std::string>>>(builder2, "El Salvador", core::List::make<std::string>("Guatemala", "Honduras"));
    core::MapBuilder::set<std::string, std::shared_ptr<std::vector<std::string>>>(builder2, "Nicaragua", core::List::make<std::string>("Costa Rica", "Honduras"));
    core::MapBuilder::set<std::string, std::shared_ptr<std::vector<std::string>>>(builder2, "Panama", core::List::make<std::string>("Colombia", "Costa Rica"));
    try {
      core::MapBuilder::remove<std::string, std::shared_ptr<std::vector<std::string>>>(builder2, "Honduras");
    } catch (core::TemperBubble const &) {
    }
    std::shared_ptr<std::vector<std::string>> was = core::MapBuilder::remove<std::string, std::shared_ptr<std::vector<std::string>>>(builder2, "El Salvador");
    core::Console::log(temper_globals::console.value, "Was there: " + core::List::join(was, ", ", [](std::string it) -> std::string { return it; }));
    std::string value4;
    try {
      value4 = core::List::get<std::string>(core::MapBuilder::remove<std::string, std::shared_ptr<std::vector<std::string>>>(builder2, "El Salvador"), 0);
    } catch (core::TemperBubble const &) {
      value4 = "Gone";
    }
    core::Console::log(temper_globals::console.value, value4);
    std::shared_ptr<core::Mapped::Ordered<std::string, std::shared_ptr<std::vector<std::string>>>> neighbors = core::Mapped::toMap<std::string, std::shared_ptr<std::vector<std::string>>>(builder2);
    core::Console::log(temper_globals::console.value, "From built: " + core::List::join(core::Mapped::get<std::string, std::shared_ptr<std::vector<std::string>>>(neighbors, "Nicaragua"), ", ", [](std::string it) -> std::string { return it; }));
    std::shared_ptr<core::Mapped::Ordered<int32_t, int32_t>> mapOfNone = core::Map::make<int32_t, int32_t>(core::List::make<std::shared_ptr<core::Pair<int32_t, int32_t>>>());
    std::shared_ptr<core::Mapped::Ordered<std::string, std::string>> mapOfOne = core::Map::make<std::string, std::string>(core::List::make<std::shared_ptr<core::Pair<std::string, std::string>>>(core::PairFactory::make<std::string, std::string>("jack", "all trades")));
    std::shared_ptr<core::Pair<int32_t, std::string>> value5 = core::PairFactory::make<int32_t, std::string>(1, "one");
    std::shared_ptr<core::Mapped::Ordered<int32_t, std::string>> mapOfTwo = core::Map::make<int32_t, std::string>(core::List::make<std::shared_ptr<core::Pair<int32_t, std::string>>>(value5, core::PairFactory::make<int32_t, std::string>(2, "electric boogaloo")));
    core::Console::log(temper_globals::console.value, "mapOfNone: " + std::to_string(core::Mapped::length<int32_t, int32_t>(mapOfNone)));
    core::Console::log(temper_globals::console.value, "mapOfOne: " + std::to_string(core::Mapped::length<std::string, std::string>(mapOfOne)));
    core::Console::log(temper_globals::console.value, "mapOfTwo: " + std::to_string(core::Mapped::length<int32_t, std::string>(mapOfTwo)));
    temper_globals::eightColors.value = core::Map::make<std::string, std::shared_ptr<std::vector<bool>>>();
    core::MapBuilder::set<std::string, std::shared_ptr<std::vector<bool>>>(temper_globals::eightColors.value, "black", core::List::make<bool>(false, false, false));
    core::MapBuilder::set<std::string, std::shared_ptr<std::vector<bool>>>(temper_globals::eightColors.value, "red", core::List::make<bool>(true, false, false));
    core::MapBuilder::set<std::string, std::shared_ptr<std::vector<bool>>>(temper_globals::eightColors.value, "green", core::List::make<bool>(false, true, false));
    core::MapBuilder::set<std::string, std::shared_ptr<std::vector<bool>>>(temper_globals::eightColors.value, "blue", core::List::make<bool>(false, false, true));
    core::MapBuilder::set<std::string, std::shared_ptr<std::vector<bool>>>(temper_globals::eightColors.value, "yellow", core::List::make<bool>(true, true, false));
    core::MapBuilder::set<std::string, std::shared_ptr<std::vector<bool>>>(temper_globals::eightColors.value, "cyan", core::List::make<bool>(false, true, true));
    core::MapBuilder::set<std::string, std::shared_ptr<std::vector<bool>>>(temper_globals::eightColors.value, "purple", core::List::make<bool>(true, false, true));
    core::MapBuilder::set<std::string, std::shared_ptr<std::vector<bool>>>(temper_globals::eightColors.value, "white", core::List::make<bool>(true, true, true));
    core::Console::log(temper_globals::console.value, "with 3 bits you can draw " + std::to_string(core::Mapped::length<std::string, std::shared_ptr<std::vector<bool>>>(temper_globals::eightColors.value)) + " colors.");
    auto fn = [](std::string color, std::shared_ptr<std::vector<bool>> bits) -> void {
      std::shared_ptr<std::vector<bool>> foundByGetOr = core::Mapped::getOr<std::string, std::shared_ptr<std::vector<bool>>>(temper_globals::eightColors.value, color, core::List::make<bool>());
      if (core::List::length<bool>(foundByGetOr) != 3) {
        core::Console::log(temper_globals::console.value, "getOr(\"" + color + "\") failed to find key");
      }
      std::string want = map::colorToHex(bits);
      std::string got = map::colorToHex(foundByGetOr);
      if (want != got) {
        core::Console::log(temper_globals::console.value, "getOr(\"" + color + "\") found " + got + " but wanted " + want);
      }
    };
    core::Mapped::forEach(temper_globals::eightColors.value, fn);
    std::shared_ptr<core::Pair<std::string, std::string>> value6 = core::PairFactory::make<std::string, std::string>("key", "value");
    std::shared_ptr<core::Mapped::Ordered<std::string, std::string>> map = core::Map::make<std::string, std::string>(core::List::make<std::shared_ptr<core::Pair<std::string, std::string>>>(value6, core::PairFactory::make<std::string, std::string>("other key", "other value")));
    map::printMap("map", map);
    std::shared_ptr<core::Mapped::Ordered<std::string, std::string>> builder = core::Mapped::toMapBuilder<std::string, std::string>(map);
    core::MapBuilder::set<std::string, std::string>(builder, "added key", "added value");
    core::MapBuilder::set<std::string, std::string>(builder, "key", "replaced value");
    map::printMapBuilder("builder", builder);
    map::printMap("map (same as above)", map);
    std::shared_ptr<core::Mapped::Ordered<std::string, std::string>> newMap = core::Mapped::toMap<std::string, std::string>(builder);
    map::printMap("builder to map", newMap);
    std::shared_ptr<core::Mapped::Ordered<std::string, std::string>> builderAsMapped = builder;
    std::shared_ptr<core::Mapped::Ordered<std::string, std::string>> newMapFromMapped = core::Mapped::toMap<std::string, std::string>(builderAsMapped);
    core::MapBuilder::remove<std::string, std::string>(builder, "other key");
    core::MapBuilder::set<std::string, std::string>(builder, "key", "double replaced value");
    map::printMap("builder to map (same as above)", newMap);
    map::printMap("builder to map via mapped (same as above)", newMapFromMapped);
    core::MapBuilder::clear<std::string, std::string>(builder);
    core::MapBuilder::set<std::string, std::string>(builder, "something", "alone");
    map::printMapBuilder("recently cleared", builder);
    std::shared_ptr<core::Mapped::Ordered<std::string, std::string>> mayOrMayNotCopyMap = core::Mapped::toMap<std::string, std::string>(map);
    std::shared_ptr<core::Mapped::Ordered<std::string, std::string>> mapped = map;
    std::shared_ptr<core::Mapped::Ordered<std::string, std::string>> mayOrMayNotCopyMapped = core::Mapped::toMap<std::string, std::string>(mapped);
    std::shared_ptr<core::Mapped::Ordered<std::string, std::string>> madeBy = core::Map::make<std::string, std::string>();
    core::MapBuilder::set<std::string, std::string>(madeBy, "temper", "temper systems");
    map::printMapBuilder("made by", madeBy);
    std::shared_ptr<core::Mapped::Ordered<std::string, std::string>> madeByAndJokes = core::Mapped::toMapBuilder<std::string, std::string>(madeBy);
    core::MapBuilder::set<std::string, std::string>(madeByAndJokes, "milk", "milking a cow");
    map::printMapBuilder("made by (and a joke about milk)", madeByAndJokes);
    map::printMapBuilder("made by (not changed from a change to a copy)", madeBy);
  });
}

}  // namespace map
