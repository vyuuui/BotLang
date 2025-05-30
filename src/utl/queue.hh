#pragma once

#include <array>
#include <cassert>
#include <cstdint>
#include <optional>

namespace utl {
template <typename T, std::size_t cap>
class CyclicQueue {
  union T_u {
    struct {
    } _dummy;
    T val;

    T_u() : _dummy() {}
    ~T_u() {}
  };
  std::array<T_u, cap> buf;
  std::size_t begin;
  std::size_t sz;

public:
  CyclicQueue() : begin(0), sz(0) {}

  template <typename... Args>
  void emplace(Args&&... args) {
    assert(!full());
    new (&buf[(begin + sz) % cap].val) T(std::forward<Args>(args)...);
    sz++;
  }

  void put(T&& val) {
    assert(!full());
    buf[(begin + sz) % cap].val = std::move(val);
    sz++;
  }
  void put(T const& val) {
    assert(!full());
    buf[(begin + sz) % cap].val = val;
    sz++;
  }

  void pop() {
    assert(!empty());
    begin = (begin + 1) % cap;
    sz--;
  }

  T const& peek() const {
    assert(!empty());
    return buf[begin].val;
  }
  T& peek() {
    assert(!empty());
    return buf[begin].val;
  }

  T const& get(size_t idx) const {
    assert(!empty());
    return buf[(begin + idx) % cap].val;
  }
  T& get(size_t idx) {
    assert(!empty());
    return buf[(begin + idx) % cap].val;
  }

  constexpr bool empty() const { return sz == 0; }
  constexpr bool full() const { return sz >= cap; }
  constexpr size_t size() const { return sz; }
  constexpr size_t capacity() const { return cap; }

};
} // namespace utl
