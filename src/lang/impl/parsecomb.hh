#pragma once

#include "lang/impl/ast.hh"
#include "lang/impl/lex.hh"
#include "lang/impl/token.hh"

#include <functional>
#include <optional>
#include <tuple>
#include <vector>

namespace lang {
class Parser;

struct ParseState {
  Lexer& l;
  std::optional<Err> e;
  Parser* p;

  ParseState(Lexer& l, Parser* p) : l(l), p(p) {}
};

template <typename T, template <typename...> typename U>
inline constexpr static bool is_outer_same_v = std::false_type{};
template <template <typename...> typename U, typename... Vs>
inline constexpr static bool is_outer_same_v<U<Vs...>, U> = std::true_type{};

template <typename T>
concept Optional = is_outer_same_v<T, std::optional>;

// Defines a well-formed combinator
template <typename T>
concept Combinator = requires (T t, ParseState& s) {
  typename T::Rt;
  typename T::HeadType;
  requires std::same_as<std::decay_t<decltype(T::Lookahead)>, typename T::HeadType>;
  requires std::same_as<std::decay_t<decltype(T::Skippable)>, bool>;
  { t.run(s) } -> Optional;
};

template <typename T>
constexpr bool operator==(std::optional<T> o, T v) {
  return o.has_value() && o.value() == v;
}

template <std::size_t idx = 0, typename... Ts>
constexpr bool eq_helper(std::optional<Token> lhs, std::tuple<Ts...> rhs) {
  if constexpr (idx < sizeof...(Ts)) {
    return lhs == std::get<idx>(rhs) || eq_helper<idx + 1>(lhs, rhs);
  } else {
    return false;
  }
}

template <typename... Ts>
constexpr bool eq_any(std::optional<Token> lhs, std::tuple<Ts...> rhs) {
  return eq_helper(lhs, rhs);
}

template <Combinator T0, Combinator... Ts>
constexpr auto lookahead_seq() {
  if constexpr (T0::Skippable && sizeof...(Ts) > 0) {
    return std::tuple_cat(T0::Lookahead, lookahead_seq<Ts...>());
  } else {
    return T0::Lookahead;
  }
}

template <typename Rt, typename... Args>
constexpr Rt rtype(Rt(*)(Args...));
template <typename C, typename Rt, typename... Args>
constexpr Rt rtype(Rt(C::*)(Args...));

template <typename T>
struct safe_void { using type = T; };
template <>
struct safe_void<void> { using type = std::monostate; };

template <typename T>
using safe_void_t = safe_void<T>::type;

template <Token Tl, Token Th>
constexpr auto gen_token_range_incl() {
  if constexpr (static_cast<int>(Tl) <= static_cast<int>(Th)) {
    return std::tuple_cat(std::tuple<Token>(Tl),
                          gen_token_range_incl<static_cast<Token>(static_cast<int>(Tl) + 1), Th>());
  } else {
    return std::tuple<>();
  }
}

template <Token T>
struct Match {
  using Rt = std::monostate;

  using HeadType = std::tuple<Token>;
  constexpr inline static HeadType Lookahead = T;
  constexpr inline static bool Skippable = false;

  static std::optional<std::monostate> run(ParseState& s) {
    if (s.l.peek() == T) {
      s.l.eat();
      return std::monostate{};
    }
    s.e = s.l.err_at_head(std::string("Expected ") + tokstr(T));
    return std::nullopt;
  }
};

template <Token T, auto Func>
struct Extract {
  static_assert(T == Token::Identifier ||
                T == Token::IntegralLiteral ||
                T == Token::FloatLiteral ||
                T == Token::DoubleLiteral ||
                T == Token::StringLiteral, "Invalid token type marked for extraction");
  using Rt = safe_void_t<decltype(rtype(Func))>;
  constexpr inline static bool IsMonostate = std::is_same_v<Rt, std::monostate>;

  using HeadType = std::tuple<Token>;
  constexpr inline static HeadType Lookahead = T;
  constexpr inline static bool Skippable = false;

  static std::optional<Rt> run(ParseState& s) {
    if (s.l.peek() == T) {
      if constexpr (IsMonostate) {
        Func(s, *s.l.peek_data());
        s.l.eat();
        return std::monostate{};
      } else {
        Rt leaf = std::move(Func(s, *s.l.peek_data()));
        s.l.eat();
        return std::move(leaf);
      }
    }
    s.l.err_at_head(std::string("Expected ") + tokstr(T));
    return std::nullopt;
  }
};

template <Token Tl, Token Th, typename ErrStr, auto Func>
struct MatchRange {
  static_assert(static_cast<int>(Tl) < static_cast<int>(Th), "Expected valid token range");
  using Rt = safe_void_t<decltype(rtype(Func))>;
  constexpr inline static bool IsMonostate = std::is_same_v<Rt, std::monostate>;

  using HeadType = decltype(gen_token_range_incl<Token::FirstKeyword, Token::LastKeyword>());
  constexpr inline static HeadType Lookahead = gen_token_range_incl<Token::FirstKeyword, Token::LastKeyword>();
  constexpr inline static bool Skippable = false;

  static std::optional<Rt> run(ParseState& s) {
    std::optional<Token> mtok = s.l.peek();
    if (!mtok) {
      return std::nullopt;
    }
    Token tok = *mtok;
    if (static_cast<int>(tok) >= static_cast<int>(Tl) &&
        static_cast<int>(tok) <= static_cast<int>(Th)) {
      if constexpr (IsMonostate) {
        Func(s, tok);
        s.l.eat();
        return std::monostate{};
      } else {
        Rt leaf = std::move(Func(s, tok));
        s.l.eat();
        return std::move(leaf);
      }
    }
    s.e = s.l.err_at_head(ErrStr::str());
    return std::nullopt;
  }
};

template <Combinator Sub, Token... Inter>
struct Intercalate {
  using Rt = std::vector<typename Sub::Rt>;

  using HeadType = typename Sub::HeadType;
  constexpr inline static HeadType Lookahead = Sub::Lookahead;
  constexpr inline static bool Skippable = true;

  using InterType = decltype(std::make_tuple(Inter...));
  constexpr inline static InterType InterSet = std::make_tuple(Inter...);

  static std::optional<Rt> run(ParseState& s) {
    Rt list;
    if (!eq_any(s.l.peek(), Lookahead)) {
      return std::move(list);
    }

    std::optional<typename Sub::Rt> sub0 = Sub::run(s);
    // Failure already reported by child combinator
    if (!sub0) {
      return std::nullopt;
    }
    list.emplace_back(std::move(*sub0));

    while (eq_any(s.l.peek(), InterSet)) {
      const Token i_type = *s.l.peek();
      s.l.eat();
      std::optional<typename Sub::Rt> sub = Sub::run(s);
      // Failure already reported by child combinator
      if (!sub) {
        return std::nullopt;
      }
      list.emplace_back(std::move(*sub));
    }
    return std::move(list);
  }
};

template <Combinator Sub, auto Func, Token... Pre>
struct PrefixRepeat {
  using Rt = typename Sub::Rt;
  static_assert(std::is_same_v<Rt, typename Sub::Rt>, "Parser callout must accept child type");

  using PrefixType = decltype(std::make_tuple(Pre...));
  constexpr inline static PrefixType PreSet = std::make_tuple(Pre...);

  using HeadType = decltype(std::tuple_cat(PreSet, Sub::Lookahead));
  constexpr inline static HeadType Lookahead = std::tuple_cat(PreSet, Sub::Lookahead);
  constexpr inline static bool Skippable = Sub::Skippable;

  static std::optional<Rt> run(ParseState& s) {
    std::vector<Token> tok_stack;
    if (eq_any(s.l.peek(), PreSet)) {
      Token match = *s.l.peek();
      s.l.eat();
      tok_stack.push_back(match);
    }

    std::optional<Rt> sub = std::move(Sub::run(s));
    if (!sub) {
      return std::nullopt;
    }

    Rt chain = std::move(*sub);
    for (auto it = tok_stack.rbegin(); it != tok_stack.rend(); it++) {
      chain = std::move(Func(s, std::move(chain), std::move(*it)));
    }
    return std::move(chain);
  }
};

template <Combinator Sub, auto Func, Token... Inf>
struct InfixRepeat {
  using Rt = typename Sub::Rt;
  static_assert(std::is_same_v<Rt, typename Sub::Rt>, "Parser callout must accept child type");

  using InfixType = decltype(std::make_tuple(Inf...));
  constexpr inline static InfixType InfSet = std::make_tuple(Inf...);

  using HeadType = typename Sub::HeadType;
  constexpr inline static HeadType Lookahead = Sub::Lookahead;
  constexpr inline static bool Skippable = Sub::Skippable;

  static std::optional<Rt> run(ParseState& s) {
    std::optional<Rt> sub0 = std::move(Sub::run(s));
    // Failure already reported by child combinator
    if (!sub0) {
      return std::nullopt;
    }

    Rt l_sub = std::move(*sub0);
    while (eq_any(s.l.peek(), InfSet)) {
      const Token i_type = *s.l.peek();
      s.l.eat();
      std::optional<Rt> sub = std::move(Sub::run(s));
      // Failure already reported by child combinator
      if (!sub) {
        return std::nullopt;
      }
      l_sub = std::move(Func(s, std::move(l_sub), std::move(*sub), i_type));
    }
    return std::move(l_sub);

  }
};



// CAUTION: does not advertise a head explicitly!
template <auto Func, Token... Head>
struct Call {
  using Rt = safe_void_t<decltype(rtype(Func))>;
  constexpr inline static bool IsMonostate = std::is_same_v<Rt, std::monostate>;

  using HeadType = decltype(std::make_tuple(Head...));
  constexpr inline static HeadType Lookahead = std::make_tuple(Head...);
  constexpr inline static bool Skippable = false;

  static std::optional<Rt> run(ParseState& s) {
    if constexpr (IsMonostate) {
      Func(s);
      if (s.e) {
        return std::nullopt;
      } else {
        return std::monostate{};
      }
    } else {
      auto res = std::move(Func(s));
      if (s.e) {
        return std::nullopt;
      } else {
        return res;
      }
    }
  }
};

template <Combinator Lhs, Combinator Rhs>
struct Chain {
  using Rt = std::conditional_t<std::is_same_v<typename Lhs::Rt, std::monostate>,
                                typename Rhs::Rt,
                                typename Lhs::Rt>;
  static_assert(std::is_same_v<typename Lhs::Rt, std::monostate> ||
                std::is_same_v<typename Rhs::Rt, std::monostate>,
                "Chained parsers both provide a result");

  using HeadType = decltype(lookahead_seq<Lhs, Rhs>());
  constexpr inline static HeadType Lookahead = lookahead_seq<Lhs, Rhs>();
  constexpr inline static bool Skippable = Lhs::Skippable && Rhs::Skippable;

  static std::optional<Rt> run(ParseState& s) {
    auto lhs = Lhs::run(s);
    if (!lhs) {
      return std::nullopt;
    }
    auto rhs = Rhs::run(s);
    if (!rhs) {
      return std::nullopt;
    }

    if constexpr (!std::is_same_v<typename Lhs::Rt, std::monostate>) {
      return lhs;
    } else if constexpr (!std::is_same_v<typename Rhs::Rt, std::monostate>) {
      return rhs;
    } else {
      return std::monostate{};
    }
  }
};

template <Combinator T0, Combinator... Ts>
struct Select {
  using Rt = typename T0::Rt;
  static_assert((std::is_same_v<typename T0::Rt, typename Ts::Rt> && ...),
                "All options must have the same return type"); 

  using HeadType = decltype(std::tuple_cat(T0::Lookahead, Ts::Lookahead...));
  constexpr inline static HeadType Lookahead = std::tuple_cat(T0::Lookahead, Ts::Lookahead...);
  constexpr inline static bool Skippable = T0::Skippable && (Ts::Skippable && ...);

  static std::optional<Rt> run(ParseState& s) {
    return run_inner<void>(s, std::make_index_sequence<sizeof...(Ts)>());
  }

  template <typename ErrStr, std::size_t... Ind>
  static std::optional<Rt> run_inner(ParseState& s, std::index_sequence<Ind...>) {
    std::optional<Rt> ret;
    // For future me:
    // Check if the next token is in the current(expanded) T's lookahead list
    // if it is, attempt a run on it and stop here
    bool res = (eq_any(s.l.peek(), T0::Lookahead) ?
                (ret = T0::run(s), true) :
                false) ||
               ((eq_any(s.l.peek(), Ts::Lookahead) ?
                 (ret = Ts::run(s), true) :
                 false) || ...);
    if (res && !ret) {
      return std::nullopt;
    }
    if constexpr(!Skippable) {
      if (!res) {
        if constexpr (std::is_void_v<ErrStr>) {
          s.e = s.l.err_at_head("Fill this shit in (query all child parsers for a generic err msg?)");
        } else {
          s.e = s.l.err_at_head(ErrStr::str());
        }
      }
    }
    return res ? ret : std::nullopt;
  }
};

template <typename Str, Combinator... Ts>
struct SelectErr : Select<Ts...> {
  static std::optional<typename Select<Ts...>::Rt> run(ParseState& s) {
    return Select<Ts...>::template run_inner<Str>(s, std::make_index_sequence<sizeof...(Ts)>());
  }
};

template <auto Func, Combinator Sub0, Combinator... Subs>
struct Collect {
  using Rt = safe_void_t<decltype(rtype(Func))>;
  constexpr inline static bool IsMonostate = std::is_same_v<Rt, std::monostate>;

  using HeadType = decltype(lookahead_seq<Sub0, Subs...>());
  constexpr inline static HeadType Lookahead = lookahead_seq<Sub0, Subs...>();
  constexpr inline static bool Skippable = Sub0::Skippable && (Subs::Skippable && ...);

  template <std::size_t Idx = 0>
  static inline bool safe_apply(ParseState& s,
                                std::tuple<typename Sub0::Rt, typename Subs::Rt...>& tup) {
    if constexpr (Idx < (sizeof...(Subs) + 1)) {
      auto res = std::decay_t<decltype(std::get<Idx>(std::tuple<Sub0, Subs...>{}))>::run(s);
      if (!res) {
        return false;
      }
      std::get<Idx>(tup) = std::move(*res);
      return safe_apply<Idx + 1>(s, tup);
    } else {
      return true;
    }
  }

  static std::optional<Rt> run(ParseState& s) {
    std::tuple<typename Sub0::Rt, typename Subs::Rt...> sub_types;
    if (!safe_apply(s, sub_types)) {
      return std::nullopt;
    }

    if constexpr (IsMonostate) {
      std::apply(Func, std::tuple_cat(std::tuple<ParseState&>(s), std::move(sub_types)));
      return std::monostate{};
    } else {
      return std::move(std::apply(Func, std::tuple_cat(std::tuple<ParseState&>(s), std::move(sub_types))));
    }
  }
};

//////////////////////
// Helper functions //
//////////////////////

// Macro for creating compile-time string types
#define STR(s) ([]() { \
                 struct ctstring { static constexpr const char* str() { return s; } }; \
                 return ctstring{}; \
               }())
// Match and eat the specified token type T
template <Token T>
constexpr Match<T> match() { return Match<T>(); }

// Match and extract the data of specified token type T.
// Valid token types for extraction are Identifier, IntegralLiteral, FloatLiteral, DoubleLiteral, StringLiteral
template <Token T, auto Func>
constexpr Extract<T, Func> extract() { return Extract<T, Func>(); }

// Matches a range of tokens, also notifying a func of the specific matched token
// ErrStr required to specify the error message if not in the range
template <Token Tl, Token Th, auto Func, typename ErrStr>
constexpr MatchRange<Tl, Th, ErrStr, Func> match_range(ErrStr) { return MatchRange<Tl, Th, ErrStr, Func>(); }

// Repeatedly run a combinator separated by any set of equal-precedence intercalator tokens
// Each sub-parse result is collected into a vector which will be the result to the parent combinator
template <Token... Inter, Combinator Sub>
constexpr Intercalate<Sub, Inter...> intercalate(Sub) { return Intercalate<Sub, Inter...>(); }

// Run a combinator which may be prefixed by a set of equal-precedence prefix tokens
template <auto Func, Token... Head, Combinator Sub>
constexpr PrefixRepeat<Sub, Func, Head...> prefix(Sub) { return PrefixRepeat<Sub, Func, Head...>(); }

// Repeatedly run a combinator separated by any set of equal-precedence infix tokens
// Each sub-parse separated by a given token results in a call to func (chains calls)
template <auto Func, Token... Inf, Combinator Sub>
constexpr InfixRepeat<Sub, Func, Inf...> infix(Sub) { return InfixRepeat<Sub, Func, Inf...>(); }

// Calls the specified function
template <auto Func, Token... Head>
constexpr Call<Func, Head...> call() { return Call<Func, Head...>(); }

// Collect a set of results from combinators into a single call
template <auto Func, Combinator... Sub>
constexpr Collect<Func, Sub...> collect(Sub...) { return Collect<Func, Sub...>(); }

// Operator for sequencing combinators
template <Combinator Lhs, Combinator Rhs>
constexpr Chain<Lhs, Rhs> operator&&(Lhs, Rhs) { return Chain<Lhs, Rhs>(); }

// Helper for defining selection of combinators wholesale
template <Combinator... Ts>
constexpr Select<Ts...> select(Ts...) { return Select<Ts...>(); }

// Helper for defining selection of combinators wholesale
template <typename Str, Combinator... Ts>
constexpr SelectErr<Str, Ts...> select_err(Str, Ts...) { return SelectErr<Str, Ts...>(); }

// Operators for defining selections piecewise (makes the compiler work a bit harder)
template <Combinator Lhs, Combinator Rhs>
constexpr Select<Lhs, Rhs> operator||(Lhs, Rhs) { return Select<Lhs, Rhs>(); }

// Operators for defining selections piecewise (makes the compiler work a bit harder)
template <Combinator T0, Combinator... Ts>
constexpr Select<T0, Ts...> operator||(T0, Select<Ts...>) { return Select<T0, Ts...>(); }

// Operators for defining selections piecewise (makes the compiler work a bit harder)
template <Combinator Tk, Combinator... Ts>
constexpr Select<Ts..., Tk> operator||(Select<Ts...>, Tk) { return Select<Ts..., Tk>(); }

// Operators for defining selections piecewise (makes the compiler work a bit harder)
template <Combinator... Tsl, Combinator... Tsr>
constexpr Select<Tsl..., Tsr...> operator||(Select<Tsl...>, Select<Tsr...>) { return Select<Tsl..., Tsr...>(); }
} // namespace lang
