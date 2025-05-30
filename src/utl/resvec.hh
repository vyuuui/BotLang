#pragma once

#include <array>
#include <cstdint>
#include <type_traits>
#include <utility>

namespace utl {
template <typename T, std::size_t N>
class reserved_vector {
private:
  using ptrdiff_t = std::ptrdiff_t;
  using size_t = std::size_t;

private:
  // Utilities
  union alignas(T) Tu {
    struct {
    } _dummy;
    T _value;
    Tu() : _dummy() {}
    ~Tu() {}
  };

  T& _get(ptrdiff_t idx) { return storage[idx]._value; }
  T const& _get(ptrdiff_t idx) const { return storage[idx]._value; }

private:
  // Data Members
  size_t sz;
  std::array<Tu, N> storage;

  template <typename Tp>
  static void destroy(Tp& v) {
    if constexpr (std::is_destructible_v<Tp> && !std::is_trivially_destructible_v<Tp>) {
      v.Tp::~Tp();
    }
  }

public:
  using iterator = std::array<T, N>::iterator;
  using const_iterator = std::array<T, N>::const_iterator;

public:
  reserved_vector() : sz(0) {}
  // GHETTO: i hate this
  reserved_vector(std::initializer_list<T> xs) {
    for (T x : xs) {
      push_back(x);
    }
  }

  void push_back(T const& v) {
    new (&_get(sz)) T(v);
    ++sz;
  }

  void push_back(T&& v) {
    new (&_get(sz)) T(std::move(v));
    ++sz;
  }

  template <typename... TArgs>
  T& emplace_back(TArgs... args) {
    new (&_get(sz)) T(args...);

    ++sz;
    return _get(sz);
  }

  void pop_back() {
    --sz;
    destroy(_get(sz));
  }

  iterator insert(iterator target_pos, T const& val) {
    if (target_pos == end()) {
      push_back(val);
    } else {
      new (&_get(sz)) T(_get(sz - 1));
      for (auto it = end() - 1; it != target_pos; --it) {
        *it = std::forward<T>(*(it - 1));
      }
      *target_pos = val;
      ++sz;
    }
    return target_pos;
  }

  iterator insert(iterator target_pos, T&& val) {
    if (target_pos == end()) {
      push_back(val);
    } else {
      new (&_get(sz)) T(_get(sz - 1));
      for (auto it = end() - 1; it != target_pos; --it) {
        *it = std::forward<T>(*(it - 1));
      }
      *target_pos = std::forward<T>(val);
      ++sz;
    }
    return target_pos;
  }

  void resize(size_t size) {
    if (size > sz) {
      for (size_t i = sz; i < size; ++i) {
        new (&_get(i)) T();
      }
      sz = size;
    } else if (size < sz) {
      if constexpr (std::is_destructible_v<T> && !std::is_trivially_destructible_v<T>) {
        for (size_t i = size; i < sz; ++i) {
          destroy(_get(i));
        }
      }
      sz = size;
    }
  }

  iterator erase(iterator pos) {
    for (auto it = pos + 1; it != end(); ++it) {
      *(it - 1) = std::forward<T>(*it);
    }
    --sz;
    destroy(_get(sz));
    return pos;
  }

  void pop_front() {
    if (sz != 0) {
      erase(begin());
    }
  }

  void clear() { resize(0); }

  size_t size() const { return sz; }
  bool empty() const { return sz == 0; }
  constexpr size_t capacity() const { return N; }
  T const* data() const { return addressof(_get(0)); }
  T* data() { return addressof(_get(0)); }

  T& back() { return _get(sz - 1); }
  T& front() { return _get(0); }
  T const& back() const { return _get(sz - 1); }
  T const& front() const { return _get(0); }

  iterator begin() { return reinterpret_cast<std::array<T, N>*>(static_cast<void*>(&storage))->begin(); }
  iterator end() { return reinterpret_cast<std::array<T, N>*>(static_cast<void*>(&storage))->begin() + sz; }
  const_iterator begin() const {
    return reinterpret_cast<std::array<T, N> const*>(static_cast<void const*>(&storage))->cbegin();
  }
  const_iterator end() const {
    return reinterpret_cast<std::array<T, N> const*>(static_cast<void const*>(&storage))->cbegin() + sz;
  }

  T& operator[](size_t idx) { return _get(idx); }
  T const& operator[](size_t idx) const { return _get(idx); }

  ~reserved_vector() {
    if constexpr (std::is_destructible_v<T> && !std::is_trivially_destructible_v<T>) {
      for (size_t i = 0; i < sz; ++i) {
        destroy(_get(i));
      }
    }
  }
};
} // namespace utl
