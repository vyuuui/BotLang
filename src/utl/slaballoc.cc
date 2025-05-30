#include "utl/slaballoc.hh"

#if defined(__linux__)
#include <sys/mman.h>
#endif

#if defined(_WIN32)
#error Implement Windows page allocation
#endif

namespace utl {
namespace {
#if defined(__linux__)
void* page_alloc_linux() {
  return mmap(nullptr, kPageSize, PROT_READ | PROT_WRITE,
              MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
}

void page_free_linux(void* page) {
  munmap(reinterpret_cast<void*>(page_rounddown(reinterpret_cast<std::uintptr_t>(page))), kPageSize);
}
#endif

#if defined(_WIN32)
#error Implement Windows page allocation
void* page_alloc_windows() { return nullptr; }

void page_free_windows(void*) {}
#endif
} // namespace

void* page_alloc() {
#if defined(__linux__)
  return page_alloc_linux();
#elif defined(_WIN32)
  return page_alloc_windows();
#endif
}

void page_free(void* page) {
#if defined(__linux__)
  return page_free_linux(page);
#elif defined(_WIN32)
  return page_free_windows();
#endif
}
} // namespace utl
