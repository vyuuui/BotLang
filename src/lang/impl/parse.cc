#include "lang/impl/parse.hh"

#include "lang/impl/lex.hh"
#include "lang/impl/parsecomb.hh"
#include "utl/slaballoc.hh"

namespace lang {
class Parser {
private:
  Lexer l;
  ParseState p;
  utl::SlabAllocator<BTNode> allocator;

  template <typename T>
  BTNode* alloc(bt::Type tp, T&& val) {
    BTNode* n = allocator.alloc();
    *n = BTNode {
      nullptr, nullptr, tp, std::forward<T>(val)
    };
    return n;
  }

  static BTNode* parse_action(ParseState& s);
  static BTNode* parse_expression(ParseState& s);

  static BTNode* ident_node(ParseState& s, TokenData const* data) {
    return s.p->alloc(bt::Type::Id, std::get<std::string>(data->val));
  }

  static BTNode* int_node(ParseState& s, TokenData const* data) {
    return s.p->alloc(bt::Type::Constant, bt::Literal{std::get<uint64_t>(data->val)});
  }

  static BTNode* flt_node(ParseState& s, TokenData const* data) {
    return s.p->alloc(bt::Type::Constant, bt::Literal{std::get<float>(data->val)});
  }

  static BTNode* dbl_node(ParseState& s, TokenData const* data) {
    return s.p->alloc(bt::Type::Constant, bt::Literal{std::get<double>(data->val)});
  }

  static BTNode* str_node(ParseState& s, TokenData const* data) {
    return s.p->alloc(bt::Type::Constant, bt::Literal{std::get<std::string>(data->val)});
  }

  static BTNode* pfx_node(ParseState& s, BTNode* sub, Token tok) {
    BTNode* n;
    switch (tok) {
      case Token::Minus:
        n = s.p->alloc(bt::Type::UnaryOp, bt::UnOp::Neg);
        break;
      case Token::BitNot:
        n = s.p->alloc(bt::Type::UnaryOp, bt::UnOp::BitNot);
        break;
      case Token::Not:
        n = s.p->alloc(bt::Type::UnaryOp, bt::UnOp::Not);
        break;
      default:
        n = nullptr;
        break;
    }
    n->first_child = sub;
    return n;
  }

  static BTNode* inf_node(ParseState& s, BTNode* lhs, BTNode* rhs, Token tok) {
    BTNode* n;
    switch (tok) {
      case Token::Or:
        n = s.p->alloc(bt::Type::BinaryOp, bt::BinOp::Or);
        break;
      case Token::And:
        n = s.p->alloc(bt::Type::BinaryOp, bt::BinOp::And);
        break;
      case Token::BitOr:
        n = s.p->alloc(bt::Type::BinaryOp, bt::BinOp::BitOr);
        break;
      case Token::BitXor:
        n = s.p->alloc(bt::Type::BinaryOp, bt::BinOp::BitXor);
        break;
      case Token::BitAnd:
        n = s.p->alloc(bt::Type::BinaryOp, bt::BinOp::BitAnd);
        break;
      case Token::Equal:
        n = s.p->alloc(bt::Type::BinaryOp, bt::BinOp::Equal);
        break;
      case Token::NotEq:
        n = s.p->alloc(bt::Type::BinaryOp, bt::BinOp::NotEqual);
        break;
      case Token::Greater:
        n = s.p->alloc(bt::Type::BinaryOp, bt::BinOp::Greater);
        break;
      case Token::Less:
        n = s.p->alloc(bt::Type::BinaryOp, bt::BinOp::Less);
        break;
      case Token::GreaterEq:
        n = s.p->alloc(bt::Type::BinaryOp, bt::BinOp::GreaterEqual);
        break;
      case Token::LessEq:
        n = s.p->alloc(bt::Type::BinaryOp, bt::BinOp::LessEqual);
        break;
      case Token::Plus:
        n = s.p->alloc(bt::Type::BinaryOp, bt::BinOp::Add);
        break;
      case Token::Minus:
        n = s.p->alloc(bt::Type::BinaryOp, bt::BinOp::Sub);
        break;
      case Token::Asterisk:
        n = s.p->alloc(bt::Type::BinaryOp, bt::BinOp::Mul);
        break;
      case Token::FSlash:
        n = s.p->alloc(bt::Type::BinaryOp, bt::BinOp::Div);
        break;
      case Token::Percent:
        n = s.p->alloc(bt::Type::BinaryOp, bt::BinOp::Mod);
        break;
      default:
        assert(false);
        break;
    }

    n->first_child = lhs;
    lhs->next_sibling = rhs;
    return n;
  }

  static std::string get_str(ParseState& s, TokenData const* data) {
    return std::get<std::string>(data->val);
  }

  static Behavior define_behavior(ParseState&, std::string&& name, std::vector<std::string>&& params, BTNode* root) {
    return Behavior {
      .name = std::move(name),
      .params = std::move(params),
      .root = root,
    };
  }

  static bt::Action kw_to_action(ParseState&, Token tok) {
    switch (tok) {
      case Token::KwIf:
        return bt::Action::If;
      case Token::KwTask:
        return bt::Action::Task;
      case Token::KwCondTask:
        return bt::Action::CondTask;
      case Token::KwSeq:
        return bt::Action::Seq;
      case Token::KwSel:
        return bt::Action::Sel;
      case Token::KwPar:
        return bt::Action::Par;
      case Token::KwCall:
        return bt::Action::Call;
      case Token::KwDelay:
        return bt::Action::Delay;
      case Token::KwOnce:
        return bt::Action::Once;
      case Token::KwRunFor:
        return bt::Action::Runfor;
      default:
        assert(false);
        return bt::Action::If;
    }
  }

  static BTNode* action_create(ParseState& s, bt::Action action, std::vector<BTNode*> nodes) {
    BTNode* act_node = s.p->alloc(bt::Type::Action, action);
    if (!nodes.empty()) {
      act_node->first_child = nodes.front();
      for (int i = 1; i < nodes.size(); i++) {
        nodes[i - 1]->next_sibling = nodes[i];
      }
    }
    return act_node;
  }

  constexpr inline static auto RecParseAction = call<Parser::parse_action,
    Token::KwIf,
    Token::KwTask,
    Token::KwCondTask,
    Token::KwSeq,
    Token::KwSel,
    Token::KwPar,
    Token::KwCall,
    Token::KwDelay,
    Token::KwOnce,
    Token::KwRunFor
  >();
  constexpr inline static auto RecParseExpr = call<Parser::parse_expression,
    Token::Identifier,
    Token::IntegralLiteral,
    Token::FloatLiteral,
    Token::DoubleLiteral,
    Token::StringLiteral,
    Token::Not,
    Token::BitNot,
    Token::Minus
  >();

  constexpr static auto parser_base() {
    constexpr auto rec =
      match<Token::LParen>() &&
      select_err(STR("Expected expression or action"),
                 match<Token::Ampersat>() && RecParseAction,
                 RecParseExpr) &&
      match<Token::RParen>();

    return select_err(
      STR("Expected identifier, constant, or expression"),
      extract<Token::Identifier, ident_node>(),
      extract<Token::IntegralLiteral, int_node>(),
      extract<Token::FloatLiteral, flt_node>(),
      extract<Token::DoubleLiteral, dbl_node>(),
      extract<Token::StringLiteral, str_node>(),
      rec);
  }

  constexpr static auto parser_unops() {
    return prefix<pfx_node, Token::Minus, Token::BitNot, Token::Not>(parser_base());
  }

  constexpr static auto parser_binops() {
    return
      infix<inf_node, Token::Or>(
       infix<inf_node, Token::And>(
        infix<inf_node, Token::BitOr>(
         infix<inf_node, Token::BitXor>(
          infix<inf_node, Token::BitAnd>(
           infix<inf_node, Token::Equal, Token::NotEq>(
            infix<inf_node, Token::Greater, Token::Less, Token::GreaterEq, Token::LessEq>(
             infix<inf_node, Token::Plus, Token::Minus>(
              infix<inf_node, Token::Asterisk, Token::FSlash, Token::Percent>(
               parser_unops())))))))));
  }

  constexpr static auto parser_action() {
    return collect<action_create>(
      match_range<Token::FirstActionKeyword, Token::LastKeyword, kw_to_action>(STR("Expected valid action")),
      intercalate<Token::Comma>(parser_binops()));
  }

  constexpr static auto parser_root() {
    constexpr auto parser_behavior =
      match<Token::KwBehavior>() &&
      collect<define_behavior>(
        match<Token::LParen>() && extract<Token::Identifier, get_str>(),
        intercalate<Token::Comma>(extract<Token::Identifier, get_str>()) &&
          match<Token::RParen>() && match<Token::Assign>(),
        parser_binops());

    constexpr auto parser = parser_behavior;
    return parser;
  }

public:
  Parser(std::istream& in) : l(in), p(l, this) {}

  std::optional<ParsedScript> parse() {
    ParsedScript script;
    for (auto tok = l.peek(); tok && *tok != Token::Eof; tok = l.peek()) {
      auto result = parser_root().run(p);
      if (!result) {
        return std::nullopt;
      }
      // TODO: handle other root parses
      if (script.behaviors.find(result->name) != script.behaviors.end()) {
        // TODO: errors and parse tree file pos annotation
      }
      script.behaviors[result->name] = std::move(*result);
    }

    script.alloc_bt = std::move(allocator);
    return script;
  }

  std::optional<Err> const& error() const {
    return p.e;
  }
};

BTNode* Parser::parse_action(ParseState& s) {
  auto res = std::move(Parser::parser_action().run(s));
  if (!res) {
    printf("Failure occurred here!\n");
    printf("Fail msg: %s\n", s.e->msg.c_str());
    // Rework this? Call catches it through checking ParseState::e, but feels unclean
    return nullptr;
  }
  return *res;
}

BTNode* Parser::parse_expression(ParseState& s) {
  auto res = std::move(Parser::parser_binops().run(s));
  if (!res) {
    // Rework this? Call catches it through checking ParseState::e, but feels unclean
    return nullptr;
  }
  return *res;
}

Result<ParsedScript> parse_input(std::istream& in) {
  Parser p(in);
  auto script = p.parse();
  if (script) {
    return Result<ParsedScript>::ok(std::move(*script));
  } else {
    return Result<ParsedScript>::err(*p.error());
  }
}
} // namespace lang
