#include "lang/impl/lex.hh"

#include "lang/impl/token.hh"
#include "utl/resvec.hh"
#include "utl/utf8.hh"

#include <vector>

namespace lang {
namespace {
struct SymTrieEntry {
  using Edge = std::pair<int, Token>;
  Token match_out;
  utl::reserved_vector<Edge, 3> edges;
  SymTrieEntry() : match_out(Token::Invalid), edges({}) {}
  SymTrieEntry(Token t) : match_out(t), edges({}) {}
  SymTrieEntry(Token t, Edge e0) : match_out(t), edges({e0}) {}
  SymTrieEntry(Token t, Edge e0, Edge e1) : match_out(t), edges({e0, e1}) {}
  SymTrieEntry(Token t, Edge e0, Edge e1, Edge e2) : match_out(t), edges({e0, e1, e2}) {}
};
SymTrieEntry kEmpty = SymTrieEntry();

std::array<SymTrieEntry, 128> kSymbolTbl0 = {
  kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty,
  kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty,
  kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty,
  SymTrieEntry(Token::Not, {'=', Token::NotEq}), // !, !=
  kEmpty, kEmpty, kEmpty,
  SymTrieEntry(Token::Percent, {'=', Token::AssignMod}), // %
  SymTrieEntry(Token::BitAnd, {'&', Token::And}), // &
  kEmpty,
  SymTrieEntry(Token::LParen), // (
  SymTrieEntry(Token::RParen), // )
  SymTrieEntry(Token::Asterisk, {'=', Token::AssignMul}), // *
  SymTrieEntry(Token::Plus, {'=', Token::AssignAdd}), // +
  SymTrieEntry(Token::Comma), // ,
  SymTrieEntry(Token::Minus, {'=', Token::AssignSub}, {'>', Token::Arrow}), // -
  SymTrieEntry(Token::Period, {'.', Token::Ellipsis}), // .
  SymTrieEntry(Token::FSlash, {'=', Token::AssignDiv}), // /
  kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty,
  SymTrieEntry(Token::Semicolon), // ;
  SymTrieEntry(Token::Less, {'=', Token::LessEq}), // <
  SymTrieEntry(Token::Assign, {'=', Token::Equal}), // =
  SymTrieEntry(Token::Greater, {'=', Token::GreaterEq}), // >
  kEmpty,
  SymTrieEntry(Token::Ampersat), // @
  kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty,
  kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty,
  kEmpty, kEmpty, kEmpty, kEmpty,
  SymTrieEntry(Token::LBracket), // [
  kEmpty,
  SymTrieEntry(Token::RBracket), // ]
  SymTrieEntry(Token::BitXor), // ^
  kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty,
  kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty,
  kEmpty, kEmpty, kEmpty, kEmpty, kEmpty, kEmpty,
  SymTrieEntry(Token::LCurly), // {
  SymTrieEntry(Token::BitOr, {'|', Token::Or}), // |
  SymTrieEntry(Token::RCurly), // }
  SymTrieEntry(Token::BitNot), // ~
  kEmpty,
};

constexpr uint8_t hex_to_nib(char digit) {
  if (digit >= '0' && digit <= '9') {
    return digit - '0';
  } else if (digit >= 'a' && digit <= 'f') {
    return digit - 'a' + 10;
  }
  return digit - 'A' + 10;
}

enum class ImmType {
  Dec, Hex, Oct, Bin, Float32, Float64, None
};

struct DfaNode {
  using TrFunc = bool(*)(int);
  using Edge = std::pair<TrFunc, size_t>;

  std::vector<Edge> edges;
  std::optional<std::string_view> failure_reason;
  ImmType type;
  bool include;
};

namespace tr {
bool dec(int ch) {
  return ch >= '0' && ch <= '9';
}
bool dec_1_9(int ch) {
  return ch >= '1' && ch <= '9';
}
bool zero(int ch) {
  return ch == '0';
}
bool prefix_x(int ch) {
  return ch == 'x';
}
bool hex (int ch) {
  return (ch >= '0' && ch <= '9') ||
         (ch >= 'a' && ch <= 'f') ||
         (ch >= 'A' && ch <= 'F');
}
bool oct(int ch) {
  return ch >= '0' && ch <= '7';
}
bool prefix_b(int ch) {
  return ch == 'b';
}
bool bin(int ch) {
  return ch == '0' || ch == '1';
}
bool period(int ch) {
  return ch == '.';
}
bool suffix_f(int ch) {
  return ch == 'f';
}
} // namespace tr

const std::vector<DfaNode> kNumericDfa = {
  DfaNode { // 0
    {
      std::make_pair(tr::dec_1_9, 1),
      std::make_pair(tr::zero, 4)
    }, "Invalid lex state", ImmType::None
  },
  DfaNode { // 1
    {
      std::make_pair(tr::dec, 1),
      std::make_pair(tr::period, 2),
    }, std::nullopt, ImmType::Dec
  },
  DfaNode { // 2
    {
      std::make_pair(tr::dec, 3),
    }, std::nullopt, ImmType::Float64
  },
  DfaNode { // 3
    {
      std::make_pair(tr::dec, 3),
      std::make_pair(tr::suffix_f, 4),
    }, std::nullopt, ImmType::Float64
  },
  DfaNode { // 4
    {}, std::nullopt, ImmType::Float32
  },
  DfaNode { // 5
    {
      std::make_pair(tr::prefix_x, 6),
      std::make_pair(tr::prefix_b, 8),
      std::make_pair(tr::oct, 10),
    }, std::nullopt, ImmType::Dec
  },
  DfaNode { // 6
    {
      std::make_pair(tr::hex, 7),
    }, "Invalid hex literal", ImmType::None
  },
  DfaNode { // 7
    {
      std::make_pair(tr::hex, 7),
    }, std::nullopt, ImmType::Hex
  },
  DfaNode { // 8
    {
      std::make_pair(tr::bin, 9),
    }, "Invalid binary literal", ImmType::None
  },
  DfaNode { // 9
    {
      std::make_pair(tr::bin, 9),
    }, std::nullopt, ImmType::Bin
  },
  DfaNode { // 10
    {
      std::make_pair(tr::oct, 10),
    }, std::nullopt, ImmType::Oct
  },
};

Result<std::pair<ImmType, std::string>> run_numeric_dfa(Utf8Reader& rdbuf) {
  DfaNode const* cur = &kNumericDfa.front();
  std::ostringstream result_build;

  bool more;
  do {
    more = false;
    for (auto const& edge : cur->edges) {
      if (edge.first(rdbuf.peek().val)) {
        result_build.put(rdbuf.peek().val);
        rdbuf.next();
        cur = &kNumericDfa[edge.second];
        more = true;
        break;
      }
    }
  } while (more);

  if (cur->failure_reason) {
    return Result<std::pair<ImmType, std::string>>::err(Err(rdbuf.cursorb(), rdbuf.cursore(), std::string(*cur->failure_reason)));
  }

  return Result<std::pair<ImmType, std::string>>::ok(std::make_pair(cur->type, result_build.str()));
}
} // namespace

Utf8Reader::Utf8Reader(std::istream& iobuf) : iobuf(iobuf), peekbuf(EOF, 0), is_bad(false), cur(0, 1, 1), scan(0, 1, 1) {
  next();
}

void Utf8Reader::next() {
  // If we're already broken, make sure we force the buffer to stay EOF
  if (is_bad) {
    peekbuf.val = EOF;
    peekbuf.nbytes = 0;
    return;
  }

  if (peekbuf.val != EOF) {
    scan.off += peekbuf.nbytes;
    if (peekbuf.val == '\n') {
      scan.line++;
      scan.col = 1;
    } else {
      scan.col++;
    }
  }

  auto m_ch = utl::utf8_read(iobuf);
  if (!m_ch) {
    is_bad = true;
    peekbuf.val = EOF;
    peekbuf.nbytes = 0;
  } else {
    peekbuf = *m_ch;
  }
}

void Lexer::skip_to_non_ws() {
  while (isspace(rdbuf.peek().val)) {
    rdbuf.next();
  }
  rdbuf.movecursor();
}

void Lexer::lex_new() {
  bool again;
  do {
    again = false;
    skip_to_non_ws();
    utl::Codepoint head = rdbuf.peek();

    if (head.val == EOF) {
      peek_queue.emplace(Token::Eof, rdbuf.cursorb(), rdbuf.cursore());
    } else if (isalpha(head.val) || head.val == '_') {
      lex_ident();
    } else if (isdigit(head.val)) {
      lex_numlit();
    } else if (head.val == '"') {
      lex_stringlit();
    } else {
      again = lex_other();
    }
  } while (again);
}

void Lexer::lex_ident() {
  std::ostringstream out_ident;
  while (isalpha(rdbuf.peek().val) || rdbuf.peek().val == '_') {
    out_ident.put(rdbuf.peek().val);
    rdbuf.next();
  }
  peek_queue.emplace(Token::Identifier, out_ident.str(), rdbuf.cursorb(), rdbuf.cursore());
  rdbuf.movecursor();
}

void Lexer::lex_numlit() {
  auto res = run_numeric_dfa(rdbuf);
  if (!res) {
    lex_failure = res.err();
    rdbuf.movecursor();
    return;
  }

  std::optional<uint64_t> ival;
  std::optional<double> dval;
  std::optional<float> fval;
  switch (res->first) {
    case ImmType::Dec:
      ival = strtoull(res->second.data(), nullptr, 10);
      break;
    case ImmType::Hex:
      ival = strtoull(res->second.data(), nullptr, 16);
      break;
    case ImmType::Oct:
      ival = strtoull(res->second.data(), nullptr, 8);
      break;
    case ImmType::Bin:
      ival = strtoull(res->second.data(), nullptr, 2);
      break;
    case ImmType::Float32:
      fval = strtof(res->second.data(), nullptr);
      break;
    case ImmType::Float64:
      dval = strtod(res->second.data(), nullptr);
      break;
    default:
      break;
  }

  if (ival) {
    peek_queue.emplace(*ival, rdbuf.cursorb(), rdbuf.cursore());
  } else if (fval) {
    peek_queue.emplace(*fval, rdbuf.cursorb(), rdbuf.cursore());
  } else if (dval) {
    peek_queue.emplace(*dval, rdbuf.cursorb(), rdbuf.cursore());
  }
  rdbuf.movecursor();
}

void Lexer::lex_stringlit() {
  // skip "
  rdbuf.next();
  std::ostringstream out_str;
  while (!rdbuf.eof() && !rdbuf.bad() && rdbuf.peek().val != '"') {
    if (rdbuf.peek().val == '\\') {
      StreamPos escapeb = rdbuf.cursore();
      rdbuf.next();
      switch (rdbuf.peek().val) {
        case '\\':
          out_str.put('\\');
          break;

        case 'n':
          out_str.put('\n');
          break;

        case 'r':
          out_str.put('\r');
          break;

        case 't':
          out_str.put('\t');
          break;

        case '0':
          out_str.put('\0');
          break;

        case '"':
          out_str.put('\"');
          break;

        case 'x': {
          rdbuf.next();
          utl::Codepoint c0 = rdbuf.peek();
          rdbuf.next();
          utl::Codepoint c1 = rdbuf.peek();
          if (c0.val != EOF && c1.val != EOF) {
            if (isxdigit(c0.val) && isxdigit(c1.val)) {
              out_str.put((hex_to_nib(c0.val) << 4) | hex_to_nib(c1.val));
            } else {
              fail_now("Invalid hex escape sequence");
            }
          } // Let the else path exit the loop naturally
          break;
        }

        default:
          rdbuf.setcursor(escapeb);
          fail_now("Invalid escape sequence");
          return;
      }
    } else {
      // TODO: re-encode back as UTF8?
      out_str.put(rdbuf.peek().val);
    }

    rdbuf.next();
  }

  // Properly terminated string
  if (rdbuf.peek().val == '"') {
    rdbuf.next();
    peek_queue.emplace(Token::StringLiteral, out_str.str(), rdbuf.cursorb(), rdbuf.cursore());
    rdbuf.movecursor();
  } else {
    if (rdbuf.bad()) {
      rdbuf.movecursor();
      fail_now("Invalid UTF-8 encoding");
    } else {
      fail_now("Unterminated string");
    }
  }
}

bool Lexer::lex_other() {
  utl::Codepoint head = rdbuf.peek();
  rdbuf.next();
  if (head.val == '/' && rdbuf.peek().val == '/') {
    rdbuf.next();
    lex_comment();
    return true;
  } else if (head.val < kSymbolTbl0.size()) {
    SymTrieEntry const& symtabent = kSymbolTbl0[head.val];
    Token match_tok = symtabent.match_out;
    // Try to look deeper
    for (SymTrieEntry::Edge const& e : symtabent.edges) {
      if (rdbuf.peek().val == e.first) {
        match_tok = e.second;
        rdbuf.next();
        break;
      }
    }

    peek_queue.emplace(match_tok, rdbuf.cursorb(), rdbuf.cursore());
  } else {
    peek_queue.emplace(Token::Invalid, rdbuf.cursorb(), rdbuf.cursore());
  }
  return false;
}

void Lexer::lex_comment() {
  utl::Codepoint cur = rdbuf.peek();
  while (!rdbuf.eof() && !rdbuf.bad() && cur.val != '\n') {
    rdbuf.next();
    cur = rdbuf.peek();
  }
  if (cur.val == '\n') {
    rdbuf.next();
  }
}

void Lexer::fail_now(std::string&& message) {
  lex_failure = Err{rdbuf.cursorb(), rdbuf.cursore(), std::move(message)};
  rdbuf.movecursor();
}

Result<Token> Lexer::peek() {
  if (peek_queue.empty()) {
    lex_new();
  }
  return lex_failure ? Result<Token>::err(*lex_failure) : Result<Token>::ok(peek_queue.peek().type);
}

Result<TokenData const*> Lexer::peek_data() {
  if (peek_queue.empty()) {
    lex_new();
  }
  return lex_failure ? Result<TokenData const*>::err(*lex_failure) : Result<TokenData const*>::ok(&peek_queue.peek());
}

bool Lexer::peek_n(Token* n, size_t count) {
  if (lex_failure || count > peek_queue.capacity()) {
    return false;
  }

  while (peek_queue.size() < count) {
    lex_new();
    if (lex_failure) {
      return false;
    }
  }

  for (size_t i = 0; i < count; i++) {
    n[i] = peek_queue.get(i).type;
  }
  return true;
}

bool Lexer::peek_n_data(TokenData* n, size_t count) {
  if (lex_failure || count > peek_queue.capacity()) {
    return false;
  }

  while (peek_queue.size() < count) {
    lex_new();
    if (lex_failure) {
      return false;
    }
  }

  for (size_t i = 0; i < count; i++) {
    n[i] = peek_queue.get(i);
  }
  return true;
}

void Lexer::eat() {
  // deq has assertion
  peek_queue.pop();
}
} // namespace lang
