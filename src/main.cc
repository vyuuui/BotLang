#include "lang/compile.hh"
#include "lang/impl/lex.hh"

#include <sstream>
#include <variant>

int main() {
  std::string test = "(1.23) \"this is a string\n\" \/\/ identifier+=32&3;\nidaftercomment";
  std::istringstream iss(test);
  lang::Lexer l(iss);
  lang::Result<lang::Token> tr = l.peek();
  while (tr.has_value() && tr.val() != lang::Token::Eof) {
    lang::TokenData const& dat = *l.peek_data().val();
    printf("New token\n");
    printf("  type: %d (%s)\n", (int)dat.type, lang::tokstr(dat.type));
    printf("  val: ");
    if (std::string const* s = std::get_if<std::string>(&dat.val); s != nullptr) {
      printf("%s\n", s->data());
    } else if (uint64_t const* i = std::get_if<uint64_t>(&dat.val); i != nullptr) {
      printf("%lu\n", *i);
    } else if (float const* f = std::get_if<float>(&dat.val); f != nullptr) {
      printf("%f\n", *f);
    } else if (double const* d = std::get_if<double>(&dat.val); d != nullptr) {
      printf("%f\n", *d);
    } else {
      printf("<invalid>\n");
    }
    printf("  begin: l=%d c=%d off=%lu\n", dat.bpos.line, dat.bpos.col, dat.bpos.off);
    printf("  end: l=%d c=%d off=%lu\n", dat.epos.line, dat.epos.col, dat.epos.off);
    l.eat();
    tr = l.peek();
  }

  if (!tr.has_value()) {
    printf("Failure occurred\n");
  }
  return 0;
}
