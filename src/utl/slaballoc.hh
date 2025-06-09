#pragma once

#include <cassert>
#include <cstdint>
#include <type_traits>

// Slab layout
// +-------------+ Page Begin
// |             |
// |   Objects   |
// |             |
// +-------------|
// | Slab Header | ---+
// +-------------+    | Page end
//                    |
// +-------------+ <--+ Page begin
// |             |
// |   Objects   |
// |             |
// +-------------|
// | Slab Header | ---+
// +-------------+    | Page end
//                    |
// +-------------+ <--+
// |     ...     |
//

namespace utl {
constexpr std::size_t kPageSize = 0x1000;
constexpr std::uintptr_t page_rounddown(std::uintptr_t addr) {
  return addr & ~static_cast<std::uintptr_t>(kPageSize - 1);
}

void* page_alloc();
void page_free(void* page);

template <typename T, bool DestructOnRelease = true>
class SlabAllocator {
private:
  struct Slab;
  union Alloc {
    T obj;
    Alloc* nextfree;

    Slab* owner() {
      return reinterpret_cast<Slab*>(page_rounddown(reinterpret_cast<std::uintptr_t>(this)));
    }
  };

  struct Slab {
    struct SlabHeader {
      Alloc* free_list;
      Slab* prev;
      Slab* next;
      std::size_t usage;
    };
    
    inline static constexpr std::size_t kMaxObjects =
      (kPageSize - sizeof(SlabHeader)) / sizeof(Alloc);
    static_assert(kMaxObjects > 2, "Objects must be small enough to fit at least two per PageSize");

    SlabHeader* header() {
      return reinterpret_cast<SlabHeader*>(
        reinterpret_cast<std::uintptr_t>(this) + kPageSize - sizeof(SlabHeader));
    }

    SlabHeader const* header() const {
      return reinterpret_cast<SlabHeader const*>(
        reinterpret_cast<std::uintptr_t>(this) + kPageSize - sizeof(SlabHeader));
    }

    static Slab* create() {
      Slab* slab_mem = reinterpret_cast<Slab*>(page_alloc());
      slab_mem->init();
      return slab_mem;
    }

    void init() {
      for (std::size_t i = 0; i < kMaxObjects - 1; i++) {
        reinterpret_cast<Alloc*>(this)[i].nextfree =
         reinterpret_cast<Alloc*>(this) + i + 1;
      }
      reinterpret_cast<Alloc*>(this)[kMaxObjects - 1].nextfree = nullptr;
      header()->free_list = reinterpret_cast<Alloc*>(this);
      header()->prev = nullptr;
      header()->next = nullptr;
      header()->usage = 0;
    }

    T* alloc() {
      SlabHeader* hdr = header();
      if (hdr->free_list == nullptr) {
        return nullptr;
      }

      Alloc* newalloc = hdr->free_list;
      hdr->free_list = newalloc->nextfree;

      hdr->usage++;
      return &newalloc->obj;
    }

    // Calls destructor on obj as well
    void destroy(T* obj) {
      SlabHeader* hdr = header();
      assert(hdr->usage > 0);

      obj->~T();
      reinterpret_cast<Alloc*>(obj)->nextfree = hdr->free_list;
      hdr->free_list = reinterpret_cast<Alloc*>(obj);
      hdr->usage--;
    }

    bool is_full() const {
      return header()->free_list == nullptr;
    }

    ~Slab() {
      if constexpr (std::is_trivially_destructible_v<T> || !DestructOnRelease) {
        return;
      }
      bool destruct_tbl[kMaxObjects];
      for (std::size_t i = 0; i < kMaxObjects; i++) {
        destruct_tbl[i] = true;
      }

      Alloc* free_ent = header()->free_list;
      Alloc* first_obj = reinterpret_cast<Alloc*>(this);

      while (free_ent != nullptr) {
        destruct_tbl[free_ent - first_obj] = false;
        free_ent = free_ent->nextfree;
      }

      for (std::size_t i = 0; i < kMaxObjects; i++) {
        if (destruct_tbl[i]) {
          destroy(&(first_obj + i)->obj);
        }
      }
    }
  }; // struct Slab

private:
  Slab* _free;
  Slab* _partial;
  Slab* _full;

private:
  void free_internal() {
    while (_free != nullptr) {
      _free->~Slab();
      Slab* oldf = _free;
      ll_remove(&_free, _free);
      page_free(oldf);
    }
    while (_partial != nullptr) {
      _partial->~Slab();
      Slab* oldp = _partial;
      ll_remove(&_partial, _partial);
      page_free(oldp);
    }
    while (_full != nullptr) {
      _full->~Slab();
      Slab* oldf = _full;
      ll_remove(&_full, _full);
      page_free(oldf);
    }
  }

  T* alloc_partial() {
    T* ret = _partial->alloc();
    if (_partial->is_full()) {
      Slab* oldp = _partial;
      ll_remove(&_partial, oldp);
      ll_insert(&_full, oldp);
    }
    return ret;
  }

  T* alloc_free() {
    T* ret = _free->alloc();
    Slab* oldf = _free;
    ll_remove(&_free, oldf);
    ll_insert(&_partial, oldf);
    return ret;
  }

  T* alloc_new() {
    Slab* new_slab = Slab::create();
    ll_insert(&_free, new_slab);
    return alloc_free();
  }

  void ll_remove(Slab** ll_head, Slab* slab) {
    Slab* next = slab->header()->next;
    Slab* prev = slab->header()->prev;
    if (prev != nullptr) {
      prev->header()->next = next;
    }
    if (next != nullptr) {
      next->header()->prev = prev;
    }

    // Also implies prev == nullptr
    if (slab == *ll_head) {
      *ll_head = next;
    }
    slab->header()->next = nullptr;
    slab->header()->prev = nullptr;
  }

  void ll_insert(Slab** ll_head, Slab* slab) {
    slab->header()->next = *ll_head;
    if (*ll_head != nullptr) {
      (*ll_head)->header()->prev = slab;
    }
    *ll_head = slab;
  }

public:
  SlabAllocator() : _free(nullptr), _partial(nullptr), _full(nullptr) {}
  SlabAllocator(SlabAllocator const&) = delete;
  SlabAllocator(SlabAllocator&& other)
    : _free(other._free), _partial(other._partial), _full(other._full) {
    other._free = nullptr;
    other._partial = nullptr;
    other._full = nullptr;
  }

  SlabAllocator& operator=(SlabAllocator const& rhs) = delete;
  SlabAllocator& operator=(SlabAllocator&& rhs) {
    free_internal();

    _free = rhs._free;
    _partial = rhs._partial;
    _full = rhs._full;

    rhs._free = nullptr;
    rhs._partial = nullptr;
    rhs._full = nullptr;
    return *this;
  }

  T* alloc() {
    if (_partial != nullptr) {
      return alloc_partial();
    }
    if (_free != nullptr) {
      return alloc_free();
    }
    return alloc_new();
  }

  // Double-free would ruin this
  void destroy(T* obj) {
    Slab* owner = reinterpret_cast<Alloc*>(obj)->owner();
    const bool was_full = owner->is_full();
    owner->destroy(obj);

    if (owner->header()->usage == 0) {
      ll_remove(&_partial, owner);
      ll_insert(&_free, owner);
    } else if (was_full) {
      ll_remove(&_full, owner);
      ll_insert(&_partial, owner);
    }
  }

  ~SlabAllocator() {
    free_internal();
  }
};
} // namespace utl
