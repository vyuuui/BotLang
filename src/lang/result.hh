#pragma once

#include <string>
#include <variant>

namespace lang {
struct StreamPos {
  int64_t off;
  int line;
  int col;

  StreamPos(int64_t off, int line, int col) : off(off), line(line), col(col) {}
};

struct Err {
  StreamPos posb;
  StreamPos pose;
  std::string msg;

  Err(StreamPos posb, StreamPos pose, std::string&& msg) : posb(posb), pose(pose), msg(std::move(msg)) {}

  std::string format();
};

template <typename T>
class Result {
  std::variant<Err, T> inner;

  Result(T&& v) : inner(std::move(v)) {}
  Result(Err&& e) : inner(std::move(e)) {}
  Result(T const& v) : inner(v) {}
  Result(Err const& e) : inner(e) {}

public:
  Result() = delete;

  static Result<T> ok(T&& v) {
    return Result<T>(std::move(v));
  }
  static Result ok(T const& v) {
    return Result<T>(v);
  }
  static Result err(Err&& e) {
    return Result<T>(std::move(e));
  }
  static Result err(Err const& e) {
    return Result<T>(e);
  }

  constexpr bool has_value() const {
    return std::holds_alternative<T>(inner);
  }
  constexpr operator bool() const {
    return has_value();
  }

  T& val() {
    return std::get<T>(inner);
  }
  T const& val() const {
    return std::get<T>(inner);
  }

  constexpr T& operator*() {
    return val();
  }
  constexpr T const& operator*() const {
    return val();
  }

  constexpr T* operator->() {
    return &val();
  }
  constexpr T const* operator->() const {
    return &val();
  }

  Err const& err() const {
    return std::get<Err>(inner);
  }
};
} // namespace lang
