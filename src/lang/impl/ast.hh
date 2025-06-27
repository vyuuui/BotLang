#pragma once

#include <cstdint>
#include <string>
#include <variant>
#include <vector>

//
/* BotLang BNF
 *
 * -------------------------------------BNF Root-----------------------------------
 * root             ::=
 *                      'import' <string> ';' |
 *                      <enum_def> |
 *                      <interface_def> |
 *                      <behavior_def> |
 *                      <fn_def> |
 *                      'extern' <id> '(' <param_list> ')' -> <fulltype> ';'
 *
 * enum_def         ::= 'enum' <id> (':' <enum_types>)? '{' <enum_body> '}'
 * interface_def    ::= 'interface' <id> (':' <id>)? '{' <interface_body> '}'
 * behavior_def     ::= 'behavior' '(' <id> <param_list> ')' '=' <b_expr> ';'
 * fn_def           ::= 'fn' <id> '(' <param_list> ')' '->' <id> <block>
 *
 *
 * ------------------------------------BNF Enums-----------------------------------
 * enum_body        ::= <id> ('=' <enum_expr>)? (',' <id> ('=' <enum_expr>)?)*
 * enum_expr        ::= <e_bor>
 * e_bor            ::= <e_bxor> ('|' <e_bxor>)*
 * e_bxor           ::= <e_band> ('^' <e_band>)*
 * e_band           ::= <e_shift> ('&' <e_shift>)*
 * e_shift          ::= <e_add> (['<<', '>>'] <e_add>)*
 * e_add            ::= <e_mult> (['+', '-'] <e_mult>)*
 * e_mult           ::= <e_unary> (['*', '/', '%'] <e_unary>)*
 * e_unary          ::= ['-', '~'] <e_unary> | <e_base_expr>
 * e_base_expr      ::= <id> | INTEGRAL LITERAL
 * enum_types       ::= 'i8' | 'i16' | 'i32' | 'i64' |
 *                      'u8' | 'u16' | 'u32' | 'u64'
 *
 *
 * ----------------------------------BNF Interfaces--------------------------------
 * interface_body   ::= <interface_body>*
 * interface_memb   ::= <fn_def> |
 *                      'var' <id> ':' <fulltype> '@' <int_literal> ';'
 *
 *
 * ----------------------------------BNF Behaviors---------------------------------
 * behavior_decl    ::= '(' <id> <param_list> ')'
 * b_expr_list      ::= E | <b_expr> (',' <b_expr>)*
 * b_expr           ::= <b_lor>
 * b_lor            ::= <b_land> ('||' <b_land>)*
 * b_land           ::= <b_bor> ('&&' <b_bor>)*
 * b_bor            ::= <b_bxor> ('|' <b_bxor>)*
 * b_bxor           ::= <b_band> ('^' <b_band>)*
 * b_band           ::= <b_eqcomp> ('&' <b_eqcomp>)*
 * b_eqcomp         ::= <b_ordcomp> (['==', '!='] <b_ordcomp>)*
 * b_ordcomp        ::= <b_add> (['>', '<', '>=', '<='] <b_add>)*
 * b_add            ::= <b_mult> (['+', '-'] <b_mult>)*
 * b_mult           ::= <b_unary> (['*', '/', '%'] <b_unary>)*
 * b_unary          ::= ['-', '~', '!'] <b_unary> | <b_base_expr>
 * b_base_expr      ::= '(' <b_expr> ')' |
 *                      '(' '#' <id> <b_expr_list> ')' |
 *                      <id> |
 *                      <literal>
 *
 *
 * ----------------------------------BNF Procedural--------------------------------
 * 
 * block            ::= '{' <stmt>* ('emit' <p_expr>)? '}'
 * stmt             ::= 'var' <id> ':' <fulltype> ('=' <p_expr>)? ';' |
 *                      <cflow>
 *                      'guard' <p_expr> ('->' <p_expr>)? ';' | // typecheck will validate return value
 *                      'break' ';' |
 *                      'continue' ';' |
 *                      'return' (<p_expr>)? ';' | // typecheck will validate return value
 *                      <stmt_expr> ';' |
 *                      ';'
 * stmt_expr        ::= <p_assign>
 * p_expr           ::= <cflow> |
 *                      <p_assign>
 * p_expr_list      ::= E | <p_expr> (',' <p_expr>)*
 * cflow            ::= 'if' <p_expr> <block> ('else' <block>)? |
 *                      'while' <p_expr> <block> |
 *                      'for' <id> 'in' <p_expr> <block> |
 *                      'match' <p_expr> '{' (<p_expr> '->' <block>)* '}' |
 *                      'case' '{' (<p_expr> '->' <block>)* '}'
 * p_assign         ::= <lvalue> ['=', '+=', '-=', '*=', '/=', '%=',
 *                                '&=', '^=', '|=', '~=', '<<=', '>>='] <p_assign> |
 *                      <p_lor>
 * p_lor            ::= <p_land> ('||' <p_land>)*
 * p_land           ::= <p_bor> ('&&' <p_bor>)*
 * p_bor            ::= <p_bxor> ('|' <p_bxor>)*
 * p_bxor           ::= <p_band> ('^' <p_band>)*
 * p_band           ::= <p_eqcomp> ('&' <p_eqcomp>)*
 * p_eqcomp         ::= <p_ordcomp> (['==', '!='] <p_ordcomp>)*
 * p_ordcomp        ::= <p_shift> (['>', '<', '>=', '<='] <p_shift>)*
 * p_shift          ::= <p_add> (['>>', '<<'] <p_add>)*
 * p_add            ::= <p_mult> (['+', '-'] <p_mult>)*
 * p_mult           ::= <p_unary> (['*', '/', '%'] <p_unary>)*
 * p_unary          ::= ['-', '~', '!'] <p_unary> | <p_cast>
 * p_cast           ::= <p_base_expr> (':' <fulltype>)?
 * p_base_expr      ::= '(' <p_expr> ')' |
 *                      <rvalue> |
 *                      <range>
 * lvalue           ::= <id> <lvalue_indir>*
 * lvalue_indir     ::= '.' <id> |
 *                      '[' <all_expr> ']'
 * rvalue           ::= <id> <indir>* |
 *                      <p_literal>
 * indir            ::= <lvalue_indir> |
 *                      '(' <p_expr_list> ')'
 * range            ::= '[' <p_expr> '..' <p_expr> ']'
 * primitive        ::= 'i8' | 'i16' | 'i32' | 'i64' |
 *                      'u8' | 'u16' | 'u32' | 'u64' |
 *                      'f32' | 'f64' | 'bool' | 'duration' |
 *                      'vec2' | 'vec3' | 'STATUS' | 'void'
 *                      
 *
 * p_literal        ::= <literal> | 'true' | 'false' | 
 *
 *
 * --------------------------------------Shared------------------------------------
 * param_list       ::= E | <id> ':' <fulltype> (',' <id> ':' <fulltype>)*
 * fulltype         ::= <typekind>* <basetype>
 * typekind         ::= ['list', 'maybe', 'view'] '&'? 
 * basetype         ::= '(' <fulltype> ('->' <fulltype>)* ')' |
 *                      <id> '&'? // typechecker must validate id
 * id               ::= IDENTIFIER TOKEN
 * literal          ::= ANY LITERAL TOKEN
 * int_literal      ::= INTEGRAL LITERAL
 * string           ::= STRING CONSTANT TOKEN
 */

namespace lang {
struct TypeKind {
  enum Kind : uint8_t {
    List,
    Maybe,
    View,
  } k;
  bool ref;
};
struct Fulltype {
  std::vector<TypeKind> kind;
  std::string basetype;
  bool ref;
};

struct Variable {
  std::string name;
  Fulltype type;
};

// Behavior tree
namespace bt {
enum class Type {
  Action,
  BinaryOp,
  UnaryOp,
  Id,
  Constant,
};

enum class BinOp {
  Add,
  And,
  BitAnd,
  BitOr,
  BitXor,
  Div,
  Equal,
  Greater,
  GreaterEqual,
  Less,
  LessEqual,
  Mod,
  Mul,
  NotEqual,
  Or,
  Sub,
};

enum class UnOp {
  BitNot,
  Neg,
  Not
};

using Literal = std::variant<uint64_t, float, double, std::string>;
// Action, Binary op, Unary op, Literal, Identifier
using NodeData = std::variant<BinOp, UnOp, Literal, std::string>;
} // namespace bt

// Procedural tree
namespace pt {
enum class Type {
  // Statements
  Seq,
  VarDecl,
  Guard,
  Brk,
  Cont,
  Ret,

  // Exprs
  If,
  IfElse,
  While,
  For,
  Match,
  Case,
  Assign,
  BinOp,
  UnOp,
  Cast,

  Memb,
  Subscript,
  Call,

  // Expr Leaves
  Identifier,
  Literal,
};

enum class AssignOp {
  None,
  Add,
  Sub,
  Mul,
  Div,
  Mod,
  And,
  Xor,
  Or,
  Not,
  Lsh,
  Rsh,
};

enum class BinaryOp {
  Add,
  And,
  BitAnd,
  BitOr,
  BitXor,
  Div,
  Equal,
  Greater,
  GreaterEqual,
  Less,
  LessEqual,
  Lsh,
  Mod,
  Mul,
  NotEqual,
  Or,
  Rsh,
  Sub,
};

enum class UnaryOp {
  BitNot,
  Neg,
  Not
};

using Literal = std::variant<uint64_t, float, double, bool, std::string>;

using NodeData = std::variant<
  std::monostate, // Seq, Guard, Brk, Cont, Ret, If, IfElse, While, For, Match, Case, Subscript, Call
  Variable, // VarDecl
  AssignOp, // Assign
  BinaryOp, // BinOp
  UnaryOp, // UnOp
  Fulltype, // Cast
  std::string, // Identifier, Memb
  Literal // Literal
>;
} // namespace pt

template <typename T, typename D>
struct ASTNode {
  ASTNode<T, D>* first_child;
  ASTNode<T, D>* next_sibling;
  T type;
  D ast_data;

  size_t nchildren() const {
    size_t c = 0;
    for (auto cur = first_child; cur != nullptr; cur = cur->next_sibling) {
      c++;
    }
    return c;
  }
};

using BTNode = ASTNode<bt::Type, bt::NodeData>;
using PTNode = ASTNode<pt::Type, pt::NodeData>;

} // namespace lang
