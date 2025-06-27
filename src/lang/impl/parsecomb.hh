#pragma once

#include "lang/impl/ast.hh"
#include "lang/impl/lex.hh"
#include "lang/impl/token.hh"

#include <functional>
#include <optional>
#include <tuple>
#include <vector>

namespace lang {
class StatePrivate;

struct ParseState {
  Lexer& l;
  std::optional<Err> e;
  StatePrivate* priv;

  ParseState(Lexer& l) : l(l), priv(nullptr) {}
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
  requires std::same_as<std::decay_t<decltype(T::Skippable)>, bool>;
  { t.run(s) } -> Optional;
  { t.test(s) } -> std::same_as<bool>;
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

// Run an ordered set of sub-combinator tests depending on their skippability
template <Combinator Sub0, Combinator... Subs>
constexpr bool test_all(ParseState& s) {
  if constexpr (Sub0::Skippable) {
    if constexpr (sizeof...(Subs) == 0) {
      return Sub0::test(s);
    } else {
      return Sub0::test(s) || test_all<Subs...>(s);
    }
  } else {
    return Sub0::test(s);
  }
}

template <typename T>
struct rtype : public rtype<decltype(&T::operator())> {};
template <typename Ret, typename... Args>
struct rtype<Ret(*)(Args...)> { using type = Ret; };
template <typename C, typename Ret, typename... Args>
struct rtype<Ret(C::*)(Args...)> { using type = Ret; };
template <typename C, typename Ret, typename... Args>
struct rtype<Ret(C::*)(Args...) const> { using type = Ret; };

template <typename T>
using rtype_t = typename rtype<T>::type;

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

  constexpr inline static bool Skippable = false;

  static constexpr bool test(ParseState& s) {
    return s.l.peek() == T;
  }

  static std::optional<std::monostate> run(ParseState& s) {
    if (test(s)) {
      s.l.eat();
      return std::monostate{};
    }
    s.e = s.l.err_at_head(std::string("Expected ") + tokstr(T));
    return std::nullopt;
  }
};

template <typename Str, bool Output>
struct MatchId {
  using Rt = typename std::conditional_t<Output, std::string, std::monostate>;

  constexpr inline static bool Skippable = false;

  static constexpr bool test(ParseState& s) {
    return s.l.peek() == Token::Identifier &&
           std::get<std::string>(s.l.peek_data().value()->val) == Str::str();
  }

  static std::optional<Rt> run(ParseState& s) {
    if (test(s)) {
      s.l.eat();
      if constexpr (Output) {
        return std::get<std::string>(s.l.peek_data().value()->val);
      } else {
        return std::monostate{};
      }
    }
    s.e = s.l.err_at_head(std::string("Expected keyword ") + Str::str());
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
  using Rt = safe_void_t<rtype_t<decltype(Func)>>;
  constexpr inline static bool IsMonostate = std::is_same_v<Rt, std::monostate>;

  constexpr inline static bool Skippable = false;

  static constexpr bool test(ParseState& s) {
    return s.l.peek() == T;
  }

  static std::optional<Rt> run(ParseState& s) {
    if (test(s)) {
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
    s.e = s.l.err_at_head(std::string("Expected ") + tokstr(T));
    return std::nullopt;
  }
};

template <Token... Ts>
struct ExtractSet {
  using Rt = Token;
  constexpr inline static bool IsMonostate = std::is_same_v<Rt, std::monostate>;

  constexpr inline static bool Skippable = false;

  static constexpr bool test(ParseState& s) {
    return eq_any(s.l.peek(), std::make_tuple(Ts...));
  }

  static std::optional<Rt> run(ParseState& s) {
    if (test(s)) {
      auto ret = s.l.peek();
      s.l.eat();
      return ret;
    }
    s.e = s.l.err_at_head(std::string("TODO: Expected for extractset"));
    return std::nullopt;
  }
};

template <Token T>
struct Flag {
  using Rt = bool;

  constexpr inline static bool Skippable = true;

  static constexpr bool test(ParseState& s) {
    return s.l.peek() == T;
  }

  // stupid optional
  static std::optional<Rt> run(ParseState& s) {
    if (test(s)) {
      s.l.eat();
      return true;
    }
    return false;
  }
};

template <Token T, Combinator Sub>
struct Maybe {
  using Rt = std::optional<typename Sub::Rt>;

  constexpr inline static bool Skippable = true;

  static constexpr bool test(ParseState& s) {
    return s.l.peek() == T;
  }

  // stupid optional
  static std::optional<Rt> run(ParseState& s) {
    if (test(s)) {
      s.l.eat();
      auto sub = std::move(Sub::run(s));
      if (!sub) {
        return std::nullopt;
      }
      return sub;
    }
    // No comment
    return std::make_optional<Rt>(std::nullopt);
  }
};

template <typename Str, Combinator Sub>
struct MaybeId {
  using Rt = std::optional<typename Sub::Rt>;

  constexpr inline static bool Skippable = true;

  static constexpr bool test(ParseState& s) {
    return s.l.peek() == Token::Identifier &&
           std::get<std::string>(s.l.peek_data().value()->val) == Str::str();
  }

  // stupid optional
  static std::optional<Rt> run(ParseState& s) {
    if (test(s)) {
      s.l.eat();
      auto sub = std::move(Sub::run(s));
      if (!sub) {
        return std::nullopt;
      }
      return sub;
    }
    // No comment
    return std::make_optional<Rt>(std::nullopt);
  }
};

template <Combinator Sub>
struct Repeat {
  using Rt = std::vector<typename Sub::Rt>;

  constexpr inline static bool Skippable = true;

  static constexpr bool test(ParseState& s) {
    return Sub::test(s);
  }

  static std::optional<Rt> run(ParseState& s) {
    Rt ret;
    while (test(s)) {
      std::optional<typename Sub::Rt> sub = std::move(Sub::run(s));
      if (!sub) {
        return std::nullopt;
      }
      ret.emplace_back(std::move(*sub));
    }
    return std::move(ret);
  }
};

template <Combinator Sub, Token... Inter>
struct Intercalate {
  using Rt = std::vector<typename Sub::Rt>;

  constexpr inline static bool Skippable = true;

  using InterType = decltype(std::make_tuple(Inter...));
  constexpr inline static InterType InterSet = std::make_tuple(Inter...);

  static constexpr bool test(ParseState& s) {
    return Sub::test(s);
  }

  static std::optional<Rt> run(ParseState& s) {
    Rt list;
    if (!test(s)) {
      return std::move(list);
    }

    std::optional<typename Sub::Rt> sub0 = std::move(Sub::run(s));
    // Failure already reported by child combinator
    if (!sub0) {
      return std::nullopt;
    }
    list.emplace_back(std::move(*sub0));

    while (eq_any(s.l.peek(), InterSet)) {
      const Token i_type = *s.l.peek();
      s.l.eat();
      std::optional<typename Sub::Rt> sub = std::move(Sub::run(s));
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

  constexpr inline static bool Skippable = Sub::Skippable;

  static constexpr bool test(ParseState& s) {
    return eq_any(s.l.peek(), PreSet) || Sub::test(s);
  }

  static std::optional<Rt> run(ParseState& s) {
    std::vector<Token> tok_stack;
    while (eq_any(s.l.peek(), PreSet)) {
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

  constexpr inline static bool Skippable = Sub::Skippable;

  static constexpr bool test(ParseState& s) {
    return Sub::test(s);
  }

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
  using Rt = safe_void_t<rtype_t<decltype(Func)>>;
  constexpr inline static bool IsMonostate = std::is_same_v<Rt, std::monostate>;

  constexpr inline static bool Skippable = false;

  static constexpr bool test(ParseState& s) {
    if constexpr (sizeof...(Head) > 0) {
      return eq_any(s.l.peek(), std::make_tuple(Head...));
    }
    return true;
  }

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

  constexpr inline static bool Skippable = Lhs::Skippable && Rhs::Skippable;

  static constexpr bool test(ParseState& s) {
    if constexpr (Lhs::Skippable) {
      return Lhs::test(s) && Rhs::test(s);
    }
    return Lhs::test(s);
  }

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

  constexpr inline static bool Skippable = T0::Skippable && (Ts::Skippable && ...);

  static constexpr bool test(ParseState& s) {
    return T0::test(s) || (Ts::test(s) || ...);
  }

  static std::optional<Rt> run(ParseState& s) {
    return run_inner<void>(s, std::make_index_sequence<sizeof...(Ts)>());
  }

  template <typename ErrStr, std::size_t... Ind>
  static std::optional<Rt> run_inner(ParseState& s, std::index_sequence<Ind...>) {
    std::optional<Rt> ret;
    // For future me:
    // Check if the next token is in the current(expanded) T's lookahead list
    // if it is, attempt a run on it and stop here
    bool res = (T0::test(s) ?
                (ret = T0::run(s), true) :
                false) ||
               ((Ts::test(s) ?
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
  static_assert(std::is_invocable_v<decltype(Func), ParseState&, typename Sub0::Rt, typename Subs::Rt...>,
                "Function parameters do not match parser output types");
  using Rt = safe_void_t<rtype_t<decltype(Func)>>;
  constexpr inline static bool IsMonostate = std::is_same_v<Rt, std::monostate>;

  constexpr inline static bool Skippable = Sub0::Skippable && (Subs::Skippable && ...);

  static constexpr bool test(ParseState& s) {
    return test_all<Sub0, Subs...>(s);
  }

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

template <Combinator First, Combinator Second>
struct Alternative {
  using Rt = typename First::Rt;
  static_assert((std::is_same_v<typename First::Rt, typename Second::Rt>),
                "All options must have the same return type"); 

  static constexpr bool test(ParseState& s) {
    return test_all<First, Second>(s);
  }

  static std::optional<Rt> run(ParseState& s) {
    s.l.mark();
    std::optional<typename First::Rt> res = std::move(First::run(s));
    if (!res) {
      s.l.rewind();
      s.e = std::nullopt;
      return std::move(Second::run(s));
    }
    return std::move(res);
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

// Matches a specific identifier based on the str() field of Str
template <bool Output = false, typename Str>
constexpr MatchId<Str, Output> match_id(Str) { return MatchId<Str, Output>(); }

// Match and extract the data of specified token type T.
// Valid token types for extraction are Identifier, IntegralLiteral, FloatLiteral, DoubleLiteral, StringLiteral
template <Token T, auto Func>
constexpr Extract<T, Func> extract() { return Extract<T, Func>(); }

// Extract a token matching a set of possibilities
template <Token... Ts>
constexpr ExtractSet<Ts...> extract_set() { return ExtractSet<Ts...>(); }

// Returns a bool if token T is present
template <Token T>
constexpr Flag<T> flag() { return Flag<T>(); }

// Possibly run a combinator if lookahead matches T
template <Token T, Combinator Sub>
constexpr Maybe<T, Sub> maybe(Sub) { return Maybe<T, Sub>(); }

// Possibly run a combinator if lookahead matches Str
template <typename Str, Combinator Sub>
constexpr MaybeId<Str, Sub> maybe_id(Str, Sub) { return MaybeId<Str, Sub>(); }

// Repeat a combinator 0 or more times
template <Combinator Sub>
constexpr Repeat<Sub> repeat(Sub) { return Repeat<Sub>(); }

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

// Attempt to run the first parser, and if a failure occurs recover and run the second parser
template <Combinator First, Combinator Second>
constexpr Alternative<First, Second> alternative(First, Second) { return Alternative<First, Second>(); }

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
