#pragma once

#include "lang/impl/ast.hh"
#include "lang/result.hh"
#include "utl/slaballoc.hh"

#include <istream>
#include <unordered_map>
#include <vector>

namespace lang {
struct Behavior {
  std::string name;
  std::vector<std::string> params;
  BTNode* root;
};

struct ParsedScript {
  std::unordered_map<std::string, Behavior> behaviors;
  utl::SlabAllocator<BTNode> alloc_bt;
};
Result<ParsedScript> parse_input(std::istream& in);
} // namespace lang
