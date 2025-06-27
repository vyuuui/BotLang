#include "lang/impl/parse.hh"

#include "lang/impl/lex.hh"
#include "lang/impl/parsecomb.hh"
#include "utl/slaballoc.hh"

namespace lang {
struct StatePrivate {
  Lexer* l;
  ParseState* state;
  utl::SlabAllocator<BTNode>* bt_allocator;
  utl::SlabAllocator<PTNode>* pt_allocator;

  template <typename T>
  BTNode* bt_alloc(bt::Type tp, T&& val) {
    BTNode* n = bt_allocator->alloc();
    *n = BTNode {
      nullptr, nullptr, tp, std::forward<T>(val)
    };
    return n;
  }
  template <typename T>
  PTNode* pt_alloc(pt::Type tp, T&& val) {
    PTNode* n = pt_allocator->alloc();
    *n = PTNode {
      nullptr, nullptr, tp, std::forward<T>(val)
    };
    return n;
  }
};





namespace shared {
inline std::string get_str(ParseState& s, TokenData const* data) {
  return std::get<std::string>(data->val);
}
inline uint64_t get_int(ParseState&, TokenData const* data) {
  return std::get<uint64_t>(data->val);
}

constexpr auto parser_fulltype() {
  auto to_kind = [](ParseState&, std::string&& kname, bool isref) -> TypeKind {
    TypeKind::Kind k;
    if (kname == "list") {
      k = TypeKind::List;
    } else if (kname == "maybe") {
      k = TypeKind::Maybe;
    } else if (kname == "view") {
      k = TypeKind::View;
    } else {
      assert(false);
    }
    return TypeKind {
      .k = k,
      .ref = isref,
    };
  };
  constexpr auto typekind =
    collect<to_kind>(
      match_id<true>(STR("list")) || match_id<true>(STR("maybe")) || match_id<true>(STR("view")),
      flag<Token::Ampersand>()
    );

  constexpr auto to_fulltype =
    [](ParseState&, std::vector<TypeKind>&& kinds, std::string&& basetype, bool is_ref) -> Fulltype {
    return Fulltype {
      .kind = std::move(kinds),
      .basetype = std::move(basetype),
      .ref = is_ref,
    };
  };
  return collect<to_fulltype>(
    repeat(typekind),
    extract<Token::Identifier, get_str>(),
    flag<Token::Ampersand>()
  );
}

constexpr auto _param_list() {
  constexpr auto to_param = [](ParseState&, std::string&& name, Fulltype&& tinfo) -> Param {
    return Param {
      .name = std::move(name),
      .tinfo = std::move(tinfo),
    };
  };
  return intercalate<Token::Comma>(
    collect<to_param>(extract<Token::Identifier, get_str>() && match<Token::Colon>(),
                      parser_fulltype()));
}
} // namespace shared





namespace btparse {
BTNode* callout_expr(ParseState& s);
BTNode* callout_action(ParseState& s);

BTNode* ident_node(ParseState& s, TokenData const* data) {
  return s.priv->bt_alloc(bt::Type::Id, std::get<std::string>(data->val));
}

BTNode* int_node(ParseState& s, TokenData const* data) {
  return s.priv->bt_alloc(bt::Type::Constant, bt::Literal{std::get<uint64_t>(data->val)});
}

BTNode* flt_node(ParseState& s, TokenData const* data) {
  return s.priv->bt_alloc(bt::Type::Constant, bt::Literal{std::get<float>(data->val)});
}

BTNode* dbl_node(ParseState& s, TokenData const* data) {
  return s.priv->bt_alloc(bt::Type::Constant, bt::Literal{std::get<double>(data->val)});
}

BTNode* str_node(ParseState& s, TokenData const* data) {
  return s.priv->bt_alloc(bt::Type::Constant, bt::Literal{std::get<std::string>(data->val)});
}

BTNode* pfx_node(ParseState& s, BTNode* sub, Token tok) {
  BTNode* n;
  switch (tok) {
    case Token::Minus:
      n = s.priv->bt_alloc(bt::Type::UnaryOp, bt::UnOp::Neg);
      break;
    case Token::BitNot:
      n = s.priv->bt_alloc(bt::Type::UnaryOp, bt::UnOp::BitNot);
      break;
    case Token::Not:
      n = s.priv->bt_alloc(bt::Type::UnaryOp, bt::UnOp::Not);
      break;
    default:
      assert(false);
      n = nullptr;
      break;
  }
  n->first_child = sub;
  return n;
}

BTNode* inf_node(ParseState& s, BTNode* lhs, BTNode* rhs, Token tok) {
  BTNode* n;
  switch (tok) {
    case Token::Or:
      n = s.priv->bt_alloc(bt::Type::BinaryOp, bt::BinOp::Or);
      break;
    case Token::And:
      n = s.priv->bt_alloc(bt::Type::BinaryOp, bt::BinOp::And);
      break;
    case Token::BitOr:
      n = s.priv->bt_alloc(bt::Type::BinaryOp, bt::BinOp::BitOr);
      break;
    case Token::BitXor:
      n = s.priv->bt_alloc(bt::Type::BinaryOp, bt::BinOp::BitXor);
      break;
    case Token::Ampersand:
      n = s.priv->bt_alloc(bt::Type::BinaryOp, bt::BinOp::BitAnd);
      break;
    case Token::Equal:
      n = s.priv->bt_alloc(bt::Type::BinaryOp, bt::BinOp::Equal);
      break;
    case Token::NotEq:
      n = s.priv->bt_alloc(bt::Type::BinaryOp, bt::BinOp::NotEqual);
      break;
    case Token::Greater:
      n = s.priv->bt_alloc(bt::Type::BinaryOp, bt::BinOp::Greater);
      break;
    case Token::Less:
      n = s.priv->bt_alloc(bt::Type::BinaryOp, bt::BinOp::Less);
      break;
    case Token::GreaterEq:
      n = s.priv->bt_alloc(bt::Type::BinaryOp, bt::BinOp::GreaterEqual);
      break;
    case Token::LessEq:
      n = s.priv->bt_alloc(bt::Type::BinaryOp, bt::BinOp::LessEqual);
      break;
    case Token::Plus:
      n = s.priv->bt_alloc(bt::Type::BinaryOp, bt::BinOp::Add);
      break;
    case Token::Minus:
      n = s.priv->bt_alloc(bt::Type::BinaryOp, bt::BinOp::Sub);
      break;
    case Token::Asterisk:
      n = s.priv->bt_alloc(bt::Type::BinaryOp, bt::BinOp::Mul);
      break;
    case Token::FSlash:
      n = s.priv->bt_alloc(bt::Type::BinaryOp, bt::BinOp::Div);
      break;
    case Token::Percent:
      n = s.priv->bt_alloc(bt::Type::BinaryOp, bt::BinOp::Mod);
      break;
    default:
      assert(false);
      break;
  }

  n->first_child = lhs;
  lhs->next_sibling = rhs;
  return n;
}

BTNode* act_node(ParseState& s,
                 std::string&& action,
                 std::vector<BTNode*>&& nodes) {
  BTNode* act_node = s.priv->bt_alloc(bt::Type::Action, std::move(action));
  if (!nodes.empty()) {
    act_node->first_child = nodes.front();
    for (int i = 1; i < nodes.size(); i++) {
      nodes[i - 1]->next_sibling = nodes[i];
    }
  }
  return act_node;
}

Behavior behavior_create(ParseState&,
                         std::string&& name,
                         std::vector<Param>&& params,
                         BTNode* root) {
  return Behavior {
    .name = std::move(name),
    .params = std::move(params),
    .root = root,
  };
}

///////////////////////////
// Behavior Tree Parsers //
///////////////////////////
constexpr auto RecParseAction = call<callout_action, Token::Identifier>();
constexpr auto RecParseExpr = call<callout_expr,
  Token::Identifier,
  Token::IntegralLiteral,
  Token::FloatLiteral,
  Token::DoubleLiteral,
  Token::StringLiteral,
  Token::Not,
  Token::BitNot,
  Token::Minus
>();

constexpr auto _base() {
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

constexpr auto _unops() {
  return prefix<pfx_node, Token::Minus, Token::BitNot, Token::Not>(_base());
}

constexpr auto _binops() {
  return
    infix<inf_node, Token::Or>(
     infix<inf_node, Token::And>(
      infix<inf_node, Token::BitOr>(
       infix<inf_node, Token::BitXor>(
        infix<inf_node, Token::Ampersand>(
         infix<inf_node, Token::Equal, Token::NotEq>(
          infix<inf_node, Token::Greater, Token::Less, Token::GreaterEq, Token::LessEq>(
           infix<inf_node, Token::Plus, Token::Minus>(
            infix<inf_node, Token::Asterisk, Token::FSlash, Token::Percent>(
             _unops())))))))));
}

constexpr auto _action() {
  return collect<act_node>(
    extract<Token::Identifier, shared::get_str>(),
    intercalate<Token::Comma>(_binops()));
}

BTNode* callout_expr(ParseState& s) {
  auto res = std::move(_binops().run(s));
  if (!res) {
    // Rework this? Call catches it through checking ParseState::e, but feels unclean
    return nullptr;
  }
  return *res;
}
BTNode* callout_action(ParseState& s) {
  auto res = std::move(_action().run(s));
  if (!res) {
    // Rework this? Call catches it through checking ParseState::e, but feels unclean
    return nullptr;
  }
  return *res;
}
} // namespace btparse





namespace enumparse {
Enumeration enum_create(ParseState&,
                        std::string&& name,
                        std::optional<std::string>&& type,
                        std::vector<std::pair<std::string, std::optional<uint64_t>>>&& ents) {
  Enumeration ret = Enumeration {
    .name = std::move(name),
    .basetype = type ? std::move(*type) : std::string(kDefaultEnumType),
  };
  uint64_t dest_ent_val = 0;
  for (auto&& [ent_name, ent_val] : ents) {
    if (ent_val) {
      dest_ent_val = *ent_val;
    }
    ret.ents.emplace(std::move(ent_name), dest_ent_val);
    dest_ent_val++;
  }

  return ret;
}

//////////////////
// Enum Parsers //
//////////////////
constexpr auto _enum_body() {
  constexpr auto infix_compute =
    [](ParseState&, uint64_t lhs, uint64_t rhs, Token t) -> uint64_t {
    switch (t) {
      case Token::BitOr:
        return lhs | rhs;
      case Token::BitXor:
        return lhs ^ rhs;
      case Token::Ampersand:
        return lhs & rhs;
      case Token::Lsh:
        return lhs << rhs;
      case Token::Rsh:
        return lhs >> rhs;
      case Token::Plus:
        return lhs + rhs;
      case Token::Minus:
        return lhs - rhs;
      case Token::Asterisk:
        return lhs * rhs;
      case Token::FSlash:
        return lhs / rhs;
      case Token::Percent:
        return lhs % rhs;
      default:
        assert(false);
        return 0;
    }
  };

  constexpr auto prefix_compute =
    [](ParseState&, uint64_t sub, Token t) -> uint64_t {
    switch (t) {
      case Token::Minus:
        return ~sub + 1;
      case Token::BitNot:
        return ~sub;
      default:
        assert(false);
        return 0;
    }
  };

  constexpr auto parser_enum_expr =
    infix<infix_compute, Token::BitOr>(
     infix<infix_compute, Token::BitXor>(
      infix<infix_compute, Token::Ampersand>(
       infix<infix_compute, Token::Lsh, Token::Rsh>(
        infix<infix_compute, Token::Plus, Token::Minus>(
         infix<infix_compute, Token::Asterisk, Token::FSlash, Token::Percent>(
          prefix<prefix_compute, Token::Minus, Token::BitNot>(
           extract<Token::IntegralLiteral, shared::get_int>())))))));

  constexpr auto to_id = [](ParseState&, std::string&& name, std::optional<uint64_t> val) {
    return std::pair<std::string, std::optional<uint64_t>>(std::move(name), val);
  };
  return intercalate<Token::Comma>(
    collect<to_id>(
      extract<Token::Identifier, shared::get_str>(),
      maybe<Token::Assign>(parser_enum_expr)
    )
  );
}
} // namespace enumparse





namespace ptparse {

constexpr pt::AssignOp tok_to_op(Token t) {
  switch (t) {
    case Token::Assign:
      return pt::AssignOp::None;
    case Token::AssignAdd:
      return pt::AssignOp::Add;
    case Token::AssignSub:
      return pt::AssignOp::Sub;
    case Token::AssignMul:
      return pt::AssignOp::Mul;
    case Token::AssignDiv:
      return pt::AssignOp::Div;
    case Token::AssignMod:
      return pt::AssignOp::Mod;
    case Token::AssignAnd:
      return pt::AssignOp::And;
    case Token::AssignXor:
      return pt::AssignOp::Xor;
    case Token::AssignOr:
      return pt::AssignOp::Or;
    case Token::AssignNot:
      return pt::AssignOp::Not;
    case Token::AssignLsh:
      return pt::AssignOp::Lsh;
    case Token::AssignRsh:
      return pt::AssignOp::Rsh;
    default:
      return pt::AssignOp::None;
  }
}

PTNode* callout_expr(ParseState& s);
PTNode* callout_assign(ParseState& s);

constexpr auto RecParseExpr = call<callout_expr,
  Token::Identifier,
  Token::IntegralLiteral,
  Token::FloatLiteral,
  Token::DoubleLiteral,
  Token::StringLiteral,
  Token::LParen,
  Token::Not,
  Token::BitNot,
  Token::Minus
>();

constexpr auto RecParseAssign = call<callout_assign,
  Token::Identifier,
  Token::IntegralLiteral,
  Token::FloatLiteral,
  Token::DoubleLiteral,
  Token::StringLiteral,
  Token::LParen,
  Token::Not,
  Token::BitNot,
  Token::Minus
>();

constexpr auto _lvalue() {
  constexpr auto member_to_indir = [](ParseState& s, std::string&& id) -> PTNode* {
    return s.priv->pt_alloc(pt::Type::Memb, std::move(id));
  };
  constexpr auto subscr_to_indir = [](ParseState& s, PTNode* index_expr) -> PTNode* {
    PTNode* subscr = s.priv->pt_alloc(pt::Type::Subscript, std::monostate{});
    subscr->first_child = index_expr;
    return subscr;
  };
  constexpr auto to_lvalue = [](ParseState& s, std::string&& id, std::vector<PTNode*>&& nodes) -> PTNode* {
    PTNode* id_node = s.priv->pt_alloc(pt::Type::Identifier, std::move(id));
    PTNode* prev = id_node;
    for (PTNode* c : nodes) {
      // Prepending is the correct behavior here:
      // for Memb, same as just adding node
      // for Subscript, the index expression should be the "right" child
      prev->next_sibling = c->first_child;
      c->first_child = prev;
      prev = c;
    }
    return prev;
  };
  constexpr auto parse_indir = repeat(select_err(
    collect<member_to_indir>(match<Token::Period>() && extract<Token::Identifier, shared::get_str>()),
    collect<subscr_to_indir>(match<Token::LBracket>() && RecParseExpr && match<Token::RBracket>())));

  return collect<to_lvalue>(
    extract<Token::Identifier, shared::get_str>(),
    parse_indir
  );
}

constexpr auto _lor() {
  return match<Token::LBracket>();
}

constexpr auto _assign() {
  // Current parser combinator scheme breaks down here like crazy
  // thanks to grammar ambiguity of assignments (fuck this shit notation)
  constexpr auto deal_with_assign_bullshit = [](ParseState& s) {
    constexpr auto assn_to_expr = [](ParseState& s, PTNode* lval, Token assn_op, PTNode* expr) -> PTNode* {
      PTNode* assignment = s.priv->pt_alloc(pt::Type::Assign, tok_to_op(assn_op));
      lval->next_sibling = expr;
      assignment->first_child = lval;
      return assignment;
    };
    return alternative(
      collect<assn_to_expr>(_lvalue(),
                            extract_set<Token::Assign, Token::AssignAdd, Token::AssignSub,
                                        Token::AssignMul, Token::AssignDiv, Token::AssignMod,
                                        Token::AssignAnd, Token::AssignXor, Token::AssignOr,
                                        Token::AssignNot, Token::AssignLsh, Token::AssignRsh>(),
                            RecParseAssign),
      _lor()
    );
  };
  return call<
    deal_with_assign_bullshit,
    Token::Minus,
    Token::BitNot,
    Token::Not,
    Token::LParen,
    Token::Identifier,
    Token::IntegralLiteral,
    Token::FloatLiteral,
    Token::DoubleLiteral,
    Token::StringLiteral>(); 
    
}

constexpr auto _expr() {
  return select_err(
    STR("Expected an expression"),
    _cflow(),
    _assign()
    );
}

constexpr auto _stmt() {
  return select_err(
    STR("Expected a statement"),
    _var_decl(),
    _cflow(),
    match_id(STR("guard")) && collect<guard_node>(_expr(), maybe<Token::Arrow>() && _expr()) && match<Token::Semicolon>(),
    match_id(STR("break")) && call<brk_node>() && match<Token::Semicolon>(),
    match_id(STR("continue")) && call<cont_node>() && match<Token::Semicolon>(),
    match_id(STR("return")) && 
  );
}

constexpr auto _block() {
  return
    match<Token::LCurly>() &&
    collect<to_block>(
      repeat(_stmt()),
      maybe_id(STR("emit"), _expr())) &&
    match<Token::RCurly>();
}
} // namespace ptparse


constexpr auto parser_root() {
  constexpr auto parser_import =
    match_id(STR("import")) &&
    extract<Token::Identifier, shared::get_str>() &&
    match<Token::Semicolon>();

  constexpr auto parser_enum =
    match_id(STR("enum")) &&
    collect<enumparse::enum_create>(
      extract<Token::Identifier, shared::get_str>(),
      maybe<Token::Colon>(extract<Token::Identifier, shared::get_str>()),
      match<Token::LCurly>() && enumparse::_enum_body() && match<Token::RCurly>());

  constexpr auto parser_behavior =
    match_id(STR("behavior")) &&
    collect<btparse::behavior_create>(
      match<Token::LParen>() && extract<Token::Identifier, shared::get_str>(),
      shared::_param_list() && match<Token::RParen>() && match<Token::Assign>(),
      btparse::_binops()) &&
    match<Token::Semicolon>();

  constexpr auto import_wrap =
    [](ParseState&, std::string&& i) { return RootExpr(std::move(i)); };
  constexpr auto enum_wrap =
    [](ParseState&, Enumeration&& e) { return RootExpr(std::move(e)); };
  constexpr auto behavior_wrap =
    [](ParseState&, Behavior&& b) { return RootExpr(std::move(b)); };

  constexpr auto parser = select_err(STR("Expected valid root expression"),
    collect<import_wrap>(parser_import),
    collect<enum_wrap>(parser_enum),
    collect<behavior_wrap>(parser_behavior));

  return parser;
}

Result<ParsedScript> parse_input(std::istream& in) {
  ParsedScript script;
  Lexer lex(in);

  ParseState state = ParseState(lex);
  StatePrivate priv = {.l = &lex, .state = &state, .bt_allocator = &script.alloc_bt};
  state.priv = &priv;

  for (auto tok = lex.peek(); tok && *tok != Token::Eof; tok = lex.peek()) {
    auto result = parser_root().run(state);
    if (!result) {
      // TODO: errors and parse tree file pos annotation
      return Result<ParsedScript>::err(*state.e);
    }

    if (Import* import = std::get_if<Import>(&result.value());
        import != nullptr) {
    } else if (Enumeration* enumeration = std::get_if<Enumeration>(&result.value());
               enumeration != nullptr) {
    } else if (Behavior* behavior = std::get_if<Behavior>(&result.value());
               behavior != nullptr) {
      if (script.behaviors.find(behavior->name) != script.behaviors.end()) {
        // TODO: errors and parse tree file pos annotation
      }
      script.behaviors[behavior->name] = std::move(*behavior);
    }
  }

  return Result<ParsedScript>::ok(std::move(script));
}
} // namespace lang
