#pragma once

#include <istream>
#include <optional>

namespace utl {
struct Codepoint {
  int val;
  int nbytes;
};

std::optional<Codepoint> utf8_read(std::istream& stream);
} // namespace utl
