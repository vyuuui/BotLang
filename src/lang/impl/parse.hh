#pragma once

#include "lang/impl/ast.hh"
#include "lang/result.hh"
#include "utl/slaballoc.hh"

#include <istream>
#include <string>
#include <unordered_map>
#include <vector>

namespace lang {
struct Param {
  std::string name;
  Fulltype tinfo;
};

struct Behavior {
  std::string name;
  std::vector<Param> params;
  BTNode* root;
};

constexpr const char* kDefaultEnumType = "i32";
struct Enumeration {
  std::string name;
  std::string basetype;
  std::unordered_map<std::string, uint64_t> ents;
};

using Import = std::string;

using RootExpr = std::variant<Import, Enumeration, Behavior>;

struct ParsedScript {
  std::vector<std::string> imports;
  std::unordered_map<std::string, Behavior> behaviors;
  utl::SlabAllocator<BTNode> alloc_bt;
};
Result<ParsedScript> parse_input(std::istream& in);
} // namespace lang
