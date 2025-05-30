#pragma once

#include "lang/compile.hh"
#include "lang/result.hh"
#include "lang/impl/token.hh"
#include "utl/queue.hh"
#include "utl/utf8.hh"

namespace lang {
class Utf8Reader {
  std::istream& iobuf;
  utl::Codepoint peekbuf;
  bool is_bad;

  // cur points to the last location we started at
  StreamPos cur;
  // scan points to the location of peekbuf
  StreamPos scan;

public:
  Utf8Reader(std::istream& iobuf);
  constexpr utl::Codepoint peek() const { return peekbuf; }
  constexpr bool bad() const { return is_bad; }
  void next();
  void movecursor() { cur = scan; }
  void setcursor(StreamPos pos) { cur = pos; }

  constexpr bool eof() const { return peekbuf.val == EOF; }

  StreamPos cursorb() const { return cur; }
  StreamPos cursore() const { return scan; }
};

class Lexer {
  constexpr inline static std::size_t kQueueSize = 32;

  utl::CyclicQueue<TokenData, kQueueSize> peek_queue;

  Utf8Reader rdbuf;

  std::optional<Err> lex_failure;

  void skip_to_non_ws();
  void lex_new();
  void lex_ident();
  void lex_numlit();
  void lex_stringlit();
  bool lex_other();
  void lex_comment();

  void fail_now(std::string&& message);

public:
  Lexer(std::istream& iobuf) : rdbuf(iobuf) {}

  Result<Token> peek();
  // Lifetime only persists until eat
  Result<TokenData const*> peek_data();

  bool peek_n(Token* n, size_t count);
  bool peek_n_data(TokenData* n, size_t count);

  void eat();
};
} // namespace lang
