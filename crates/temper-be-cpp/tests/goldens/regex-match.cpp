// init.hpp
#pragma once

#include <temper-core/core.hpp>
#include <std/regex.hpp>

namespace match {

// Runs the module's top level, once. Exported functions, constructors,
// static functions, and accessors run it first, so calling it is
// optional.
void global_init_init();

}  // namespace match

// init.cpp
#include <match/init.hpp>

namespace match {

namespace core = temper::core;

namespace temper_globals {

static core::GlobalSlot<std::shared_ptr<core::Console::Type>> console;
static core::GlobalSlot<std::string> text;
static core::GlobalSlot<std::string> moreText;

}  // namespace temper_globals

static void checkGroup(std::shared_ptr<temper_std::Group> const & group) {
  std::string name = group->name;
  std::string value = group->value;
  int32_t begin = group->begin;
  int32_t end = group->end;
  std::string size = std::to_string(core::String::countBetween(temper_globals::text.value, begin, end));
  std::string begin2 = std::to_string(core::String::countBetween(temper_globals::text.value, core::String::begin(), begin));
  core::Console::log(temper_globals::console.value, "match group '" + name + "': '" + value + "' at " + begin2 + " size " + size);
}

void global_init_init() {
  core::init_once([] {
    temper_globals::console.construct();
    temper_globals::text.construct();
    temper_globals::moreText.construct();
    temper_std::global_init_regex();
    temper_globals::console.value = core::Console::get_console();
    std::shared_ptr<temper_std::RegexNode> regexNode = temper_std::oneOrMore(temper_std::CodeSet::make(core::List::make<std::shared_ptr<temper_std::CodePart>>(temper_std::CodePoints::make("ab"))));
    std::shared_ptr<temper_std::Regex> regex = temper_std::Sequence::make(core::List::make<std::shared_ptr<temper_std::RegexNode>>(regexNode, temper_std::CodePoints::make("c")))->compiled();
    std::shared_ptr<temper_std::Regex> full = temper_std::entire(regex->data)->compiled();
    std::shared_ptr<std::vector<std::string>> strings = core::List::make<std::string>("abc", "ac", "bbabac", "ab", "dc", "acc", "cbc");
    {
      int32_t length = core::List::length<std::string>(strings);
      int32_t i = 0;
      while (i < length) {
        std::string string = core::List::get<std::string>(strings, i);
        i = core::Int::add(i, 1);
        core::Console::log(temper_globals::console.value, "found " + string + ": " + core::Boolean::toString(regex->found(string)));
        core::Console::log(temper_globals::console.value, "matches " + string + ": " + core::Boolean::toString(full->found(string)));
      }
    }
    std::shared_ptr<temper_std::RegexNode> value2 = temper_std::Repeat::make(temper_std::CodePoints::make("a"), 1, nullptr, false);
    std::shared_ptr<temper_std::RegexNode> value3 = temper_std::Capture::make("intro", temper_std::Repeat::make(temper_std::CodePoints::make("b"), 1, nullptr, false));
    std::shared_ptr<temper_std::RegexNode> value4 = temper_std::CodePoints::make("c");
    std::shared_ptr<temper_std::RegexNode> value5 = temper_std::Capture::make("option1", temper_std::CodePoints::make("d"));
    std::shared_ptr<temper_std::Regex> catcher = temper_std::Sequence::make(core::List::make<std::shared_ptr<temper_std::RegexNode>>(value2, value3, value4, temper_std::Repeat::make(temper_std::Or::make(core::List::make<std::shared_ptr<temper_std::RegexNode>>(value5, temper_std::Capture::make("alphaOption2", temper_std::CodePoints::make("e")))), 1, nullptr, false)))->compiled();
    temper_globals::text.value = "🌍aaabbceef";
    std::shared_ptr<temper_std::Match> match = catcher->find(temper_globals::text.value);
    match::checkGroup(match->full);
    match::checkGroup(core::Mapped::get<std::string, std::shared_ptr<temper_std::Group>>(match->groups, "intro"));
    match::checkGroup(core::Mapped::get<std::string, std::shared_ptr<temper_std::Group>>(match->groups, "alphaOption2"));
    std::shared_ptr<temper_std::Regex> thing = temper_std::Repeat::make(temper_std::CodeSet::make(core::List::make<std::shared_ptr<temper_std::CodePart>>(temper_std::CodeRange::make(97, 122)), false), 1, nullptr, false)->compiled();
    std::string multiText = "---abc---xyz---";
    std::shared_ptr<temper_std::Match> first = thing->find(multiText);
    std::shared_ptr<temper_std::Match> second = thing->find(multiText, first->full->end);
    core::Console::log(temper_globals::console.value, "Found both " + first->full->value + " and " + second->full->value + ".");
    temper_globals::moreText.value = "🌍aaabbcee🌎abcd🌏";
    auto fn = [](std::shared_ptr<temper_std::Match> match2) -> std::string {
      std::shared_ptr<core::Mapped::Ordered<std::string, std::shared_ptr<temper_std::Group>>> groups = match2->groups;
      std::string option1_2;
      try {
        option1_2 = core::Mapped::get<std::string, std::shared_ptr<temper_std::Group>>(groups, "option1")->value;
      } catch (core::TemperBubble const &) {
        option1_2 = "";
      }
      std::string alphaOption2;
      try {
        alphaOption2 = core::Mapped::get<std::string, std::shared_ptr<temper_std::Group>>(groups, "alphaOption2")->value;
      } catch (core::TemperBubble const &) {
        alphaOption2 = "";
      }
      int32_t begin2 = core::String::countBetween(temper_globals::moreText.value, core::String::begin(), match2->full->begin);
      return "-" + option1_2 + alphaOption2 + std::to_string(begin2) + "-";
    };
    std::string replaced = catcher->replace(temper_globals::moreText.value, fn);
    core::Console::log(temper_globals::console.value, "replaced: " + replaced);
    std::string notReplaced = catcher->replace("nope", [](std::shared_ptr<temper_std::Match>) -> std::string { return "hi"; });
    core::Console::log(temper_globals::console.value, "not replaced: " + notReplaced);
    std::string noMatch;
    try {
      noMatch = catcher->find("not here")->full->value;
    } catch (core::TemperBubble const &) {
      noMatch = "sorry";
    }
    core::Console::log(temper_globals::console.value, "No match says " + noMatch);
    std::shared_ptr<temper_std::RegexNode> value6 = temper_std::CodePoints::make("ab");
    std::shared_ptr<temper_std::RegexNode> or_ = temper_std::Or::make(core::List::make<std::shared_ptr<temper_std::RegexNode>>(value6, temper_std::CodePoints::make("bc")))->compiled()->data;
    std::string option1 = "ab";
    std::string option2 = "bc";
    std::string optionNone = "cb";
    core::Console::log(temper_globals::console.value, "or option 1: " + core::Boolean::toString(or_->found(option1)));
    core::Console::log(temper_globals::console.value, "or option 2: " + core::Boolean::toString(or_->found(option2)));
    core::Console::log(temper_globals::console.value, "or option none: " + core::Boolean::toString(or_->found(optionNone)));
    std::shared_ptr<temper_std::Regex> begin = temper_std::Sequence::make(core::List::make<std::shared_ptr<temper_std::RegexNode>>(temper_std::temper_globals::Begin.value, temper_std::CodePoints::make("a")))->compiled();
    std::string beginsWithATrue = "ab";
    std::string beginsWithAFalse = "bab";
    core::Console::log(temper_globals::console.value, "begin true: " + core::Boolean::toString(begin->found(beginsWithATrue)));
    core::Console::log(temper_globals::console.value, "begin false: " + core::Boolean::toString(begin->found(beginsWithAFalse)));
    std::shared_ptr<temper_std::RegexNode> value7 = temper_std::CodePoints::make("a");
    std::shared_ptr<temper_std::Regex> negatedBasic = temper_std::entire(temper_std::Sequence::make(core::List::make<std::shared_ptr<temper_std::RegexNode>>(value7, temper_std::CodeSet::make(core::List::make<std::shared_ptr<temper_std::CodePart>>(temper_std::CodeRange::make(97, 99)), true)))->compiled()->data)->compiled();
    std::string nbTrue = "a🌍";
    std::string nbFalse = "ac";
    core::Console::log(temper_globals::console.value, "negated basic true: " + core::Boolean::toString(negatedBasic->found(nbTrue)));
    core::Console::log(temper_globals::console.value, "negated basic false: " + core::Boolean::toString(negatedBasic->found(nbFalse)));
    std::string simplerLines = "It was the best of times.\nIt was the worst of times.";
    core::Console::log(temper_globals::console.value, temper_std::Repeat::make(temper_std::CodeSet::make(core::List::make<std::shared_ptr<temper_std::CodePart>>(), true), 1, nullptr, false)->compiled()->find(simplerLines)->full->value);
    std::string value8;
    try {
      value8 = temper_std::Repeat::make(temper_std::CodeSet::make(core::List::make<std::shared_ptr<temper_std::CodePart>>(), false), 1, nullptr, false)->compiled()->find(simplerLines)->full->value;
    } catch (core::TemperBubble const &) {
      value8 = "-";
    }
    core::Console::log(temper_globals::console.value, value8);
    std::shared_ptr<temper_std::CodePart> codePart = temper_std::CodeRange::make(97, 122);
    std::shared_ptr<temper_std::CodePart> value9 = temper_std::CodeRange::make(65, 90);
    std::shared_ptr<temper_std::RegexNode> idStart = temper_std::CodeSet::make(core::List::make<std::shared_ptr<temper_std::CodePart>>(codePart, value9, temper_std::CodePoints::make("_")), false)->compiled()->data;
    std::shared_ptr<temper_std::Or> idContinue = temper_std::Or::make(core::List::make<std::shared_ptr<temper_std::RegexNode>>(idStart, temper_std::CodeSet::make(core::List::make<std::shared_ptr<temper_std::CodePart>>(temper_std::CodeRange::make(48, 57)), false)->compiled()->data));
    std::shared_ptr<temper_std::Regex> word = temper_std::Sequence::make(core::List::make<std::shared_ptr<temper_std::RegexNode>>(temper_std::temper_globals::WordBoundary.value, idStart, temper_std::Repeat::make(idContinue, 0, nullptr), temper_std::temper_globals::WordBoundary.value))->compiled();
    std::string expected = "def_456";
    std::string found = word->find("123_abc " + expected + " ghi_789")->full->value;
    core::Console::log(temper_globals::console.value, "Found id: " + found);
    std::string stringToEscape = "$(*^";
    std::shared_ptr<temper_std::Regex> regexWithInterpolatedString = temper_std::Sequence::make(core::List::make<std::shared_ptr<temper_std::RegexNode>>(temper_std::temper_globals::Dot.value, temper_std::Repeat::make(temper_std::CodePoints::make(stringToEscape), 1, nullptr, false), temper_std::temper_globals::Dot.value))->compiled();
    std::string stringToEscapeAndMore = "a" + stringToEscape + stringToEscape + "b";
    std::shared_ptr<temper_std::Match> escapedMatch = regexWithInterpolatedString->find(stringToEscapeAndMore);
    core::Console::log(temper_globals::console.value, escapedMatch->full->value);
    std::shared_ptr<temper_std::Match> ownFullMatch = temper_std::Sequence::make(core::List::make<std::shared_ptr<temper_std::RegexNode>>(temper_std::temper_globals::Dot.value, temper_std::Capture::make("full", temper_std::temper_globals::Dot.value)))->compiled()->find("ab");
    std::string ownFull = core::Mapped::get<std::string, std::shared_ptr<temper_std::Group>>(ownFullMatch->groups, "full")->value;
    core::Console::log(temper_globals::console.value, "Own full: " + ownFull + " vs " + ownFullMatch->full->value);
    std::string comboLines = "It was the best\nof times.\r\n\rIt was the worst of times.";
    {
      std::shared_ptr<temper_std::RegexNode> value10 = temper_std::CodePoints::make("\n");
      std::shared_ptr<temper_std::RegexNode> value11 = temper_std::CodePoints::make("\r");
      std::shared_ptr<std::vector<std::string>> list = temper_std::Or::make(core::List::make<std::shared_ptr<temper_std::RegexNode>>(value10, temper_std::Sequence::make(core::List::make<std::shared_ptr<temper_std::RegexNode>>(value11, temper_std::Repeat::make(temper_std::CodePoints::make("\n"), 0, 1, false)))))->compiled()->split(comboLines);
      int32_t length2 = core::List::length<std::string>(list);
      int32_t i2 = 0;
      while (i2 < length2) {
        std::string line = core::List::get<std::string>(list, i2);
        i2 = core::Int::add(i2, 1);
        core::Console::log(temper_globals::console.value, line + ";");
      }
    }
  });
}

}  // namespace match
