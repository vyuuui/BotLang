#pragma once

#include "lang/result.hh"

#include <cstdint>
#include <string>

namespace lang {
enum class Token {
  // Symbols
  LParen,    // (
  RParen,    // )
  Ampersat,  // @
  Plus,      // +
  Minus,     // -
  Asterisk,  // *
  FSlash,    // /
  Percent,   // %
  And,       // &&
  Or,        // ||
  Not,       // !
  BitAnd,    // &
  BitOr,     // |
  BitXor,    // ^
  BitNot,    // ~
  Equal,     // ==
  Greater,   // >
  Less,      // <
  NotEq,     // !=
  GreaterEq, // >=
  LessEq,    // <=
  LCurly,    // {
  RCurly,    // }
  LBracket,  // [
  RBracket,  // ]
  Arrow,     // ->
  Semicolon, // ;
  Ellipsis,  // ..
  Comma,     // ,
  Period,    // .
  Assign,    // =
  AssignAdd, // +=
  AssignSub, // -=
  AssignMul, // *=
  AssignDiv, // /=
  AssignMod, // %=

  FirstSymbol = LParen,
  LastSymbol = AssignMod,

  // Literals
  IntegralLiteral,
  FloatLiteral,
  StringLiteral,
  TimeLiteral,

  FirstLiteral = IntegralLiteral,
  LastLiteral = TimeLiteral,

  // Everything else
  Identifier,
  Invalid,
  Eof,
}; // enum class Token

const char* tokstr(Token t);

struct TokenData {
  Token type;
  std::variant<std::monostate, std::string, uint64_t, float, double> val;
  StreamPos bpos;
  StreamPos epos;

  TokenData(Token type, StreamPos bpos, StreamPos epos) : type(type), val(std::monostate{}), bpos(bpos), epos(epos) {}
  TokenData(Token type, std::string&& val, StreamPos bpos, StreamPos epos) : type(type), val(std::move(val)), bpos(bpos), epos(epos) {}
  TokenData(uint64_t val, StreamPos bpos, StreamPos epos) : type(Token::IntegralLiteral), val(val), bpos(bpos), epos(epos) {}
  TokenData(float val, StreamPos bpos, StreamPos epos) : type(Token::FloatLiteral), val(val), bpos(bpos), epos(epos) {}
  TokenData(double val, StreamPos bpos, StreamPos epos) : type(Token::FloatLiteral), val(val), bpos(bpos), epos(epos) {}
};
} // namespace lang
