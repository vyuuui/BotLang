#include "utl/utf8.hh"

namespace utl {
std::optional<Codepoint> utf8_read(std::istream& stream) {
  Codepoint cp_out;
  int c0 = stream.get();
  if (c0 == EOF) {
    cp_out.val = EOF;
    cp_out.nbytes = 0;
  } else if ((c0 & 0x80) == 0x00) {
    cp_out.val = c0;
    cp_out.nbytes = 1;
  } else if ((c0 & 0xe0) == 0xc0) {
    int c1 = stream.get();
    if (c1 == EOF || (c1 & 0xc0) != 0x80) {
      return std::nullopt;
    }
    cp_out.val = ((c0 & 0b00011111) << 6) | (c1 & 0b00111111);
    cp_out.nbytes = 2;
  } else if ((c0 & 0xf0) == 0xe0) {
    int c1 = stream.get();
    int c2 = stream.get();
    if (c1 == EOF || c2 == EOF || (c1 & 0xc0) != 0x80 || (c2 & 0xc0) != 0x80) {
      return std::nullopt;
    }
    cp_out.val = ((c0 & 0b00001111) << 12) |
                  ((c1 & 0b00111111) << 6) |
                  (c2 & 0b00111111);
    cp_out.nbytes = 3;
  } else if ((c0 & 0xf8) == 0xf0) {
    int c1 = stream.get();
    int c2 = stream.get();
    int c3 = stream.get();
    if (c1 == EOF || c2 == EOF || c3 == EOF || (c1 & 0xc0) != 0x80 || (c2 & 0xc0) != 0x80 || (c3 & 0xc0) != 0x80) {
      return std::nullopt;
    }
    cp_out.val = ((c0 & 0b00000111) << 18) |
                  ((c1 & 0b00111111) << 12) |
                  ((c2 & 0b00111111) << 6) |
                  (c3 & 0b00111111);
    cp_out.nbytes = 4;
  } else {
    return std::nullopt;
  }

  return cp_out;
}
} // namespace utl
