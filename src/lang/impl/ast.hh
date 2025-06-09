#pragma once

#include <cstdint>
#include <string>
#include <variant>

//
/* BotLang BNF
 * -----------
 * root             ::= 'behavior' <behavior_decl> '=' <behavior_expr> |
 *                      'fn' <id> '(' <decl_list> ')' '->' <id> <block> |
 *                      'import' <string>
 * behavior_decl    ::= '(' <id> <id_list> ')'
 * id_list          ::= E | <id> (',' <id>)*
 * param_list       ::= E | <behavior_def> (',' <behavior_def>)*
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
 *                      '(' '#' <action> <b_expr_list> ')' |
 *                      <id> |
 *                      <constant>
 * action         ::= 'if' | 'task' | 'condtask' | 'seq' | 'sel' | 'par' |
 *                    'call' | 'delay' | 'once' | 'runfor'
 * id             ::= IDENTIFIER TOKEN
 * constant       ::= ANY CONSTANT TOKEN
 * string         ::= STRING CONSTANT TOKEN
 *
 * block          ::= ...
 * decl_list      ::= ...
 */

namespace lang {
// Behavior tree
namespace bt {
enum class Type {
  Action,
  BinaryOp,
  UnaryOp,
  Id,
  Constant,
};

enum class Action {
  Call,
  CondTask,
  Delay,
  If,
  Once,
  Par,
  Runfor,
  Sel,
  Seq,
  Task,
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
using NodeData = std::variant<Action, BinOp, UnOp, Literal, std::string>;
} // namespace bt

// Procedural tree
namespace pt {
enum class Type {
};

using NodeData = std::variant<std::monostate>;
} // namespace pt

template <typename T, typename D>
struct ASTNode {
  ASTNode<T, D>* first_child;
  ASTNode<T, D>* next_sibling;
  T type;
  D ast_data;
};

using BTNode = ASTNode<bt::Type, bt::NodeData>;
using PTNode = ASTNode<pt::Type, pt::NodeData>;

} // namespace lang
