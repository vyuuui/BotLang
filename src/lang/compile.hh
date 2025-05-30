#pragma once

#include "lang/result.hh"

#include <filesystem>
#include <istream>
#include <variant>

namespace lang {
struct CompiledScript {
};

Result<CompiledScript> compile(std::istream& stream);
Result<CompiledScript> compile(std::filesystem::path const& path);
} // namespace lang
