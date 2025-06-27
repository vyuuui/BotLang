#include "lang/compile.hh"
#include "lang/impl/lex.hh"
#include "lang/impl/parse.hh"

#include <sstream>
#include <fstream>
#include <variant>

using namespace lang;

void ptree(BTNode const* n) {
  printf("(");
  switch (n->type) {
    case bt::Type::Action:
      printf("@%s", std::get<std::string>(n->ast_data).c_str());
      break;
    case bt::Type::BinaryOp: {
      switch (std::get<bt::BinOp>(n->ast_data)) {
        case bt::BinOp::Add:
          printf("+");
          break;
        case bt::BinOp::And:
          printf("&&");
          break;
        case bt::BinOp::BitAnd:
          printf("&");
          break;
        case bt::BinOp::BitOr:
          printf("|");
          break;
        case bt::BinOp::BitXor:
          printf("^");
          break;
        case bt::BinOp::Div:
          printf("/");
          break;
        case bt::BinOp::Equal:
          printf("=");
          break;
        case bt::BinOp::Greater:
          printf(">");
          break;
        case bt::BinOp::GreaterEqual:
          printf(">=");
          break;
        case bt::BinOp::Less:
          printf("<");
          break;
        case bt::BinOp::LessEqual:
          printf("<=");
          break;
        case bt::BinOp::Mod:
          printf("%%");
          break;
        case bt::BinOp::Mul:
          printf("*");
          break;
        case bt::BinOp::NotEqual:
          printf("!=");
          break;
        case bt::BinOp::Or:
          printf("||");
          break;
        case bt::BinOp::Sub:
          printf("-");
          break;
      }
      break;
    }
    case bt::Type::UnaryOp: {
      switch (std::get<bt::UnOp>(n->ast_data)) {
        case bt::UnOp::BitNot:
          printf("~");
          break;
        case bt::UnOp::Neg:
          printf("-");
          break;
        case bt::UnOp::Not:
          printf("!");
          break;
      }
      break;
    }
    case bt::Type::Id:
      printf("%s", std::get<std::string>(n->ast_data).c_str());
      break;
    case bt::Type::Constant: {
      bt::Literal const& lit = std::get<bt::Literal>(n->ast_data);
      if (std::string const* s = std::get_if<std::string>(&lit); s != nullptr) {
        printf("\"%s\"", s->c_str());
      } else if (uint64_t const* i = std::get_if<uint64_t>(&lit); i != nullptr) {
        printf("%lu", *i);
      } else if (float const* f = std::get_if<float>(&lit); f != nullptr) {
        printf("%f", *f);
      } else if (double const* d = std::get_if<double>(&lit); d != nullptr) {
        printf("%f", *d);
      }
      break;
    }
  }
  if (n->first_child != nullptr) {
    printf(" ");
    BTNode* ch = n->first_child;
    while (ch) {
      ptree(ch);
      if (ch->next_sibling != nullptr) {
        printf(", ");
      }
      ch = ch->next_sibling;
    }
  }
  printf(")");
}

void print_behavior(Behavior const& b) {
  printf("Behavior name and params: (%s", b.name.c_str());
  if (!b.params.empty()) {
    printf(" %s", b.params.front().name.c_str());
    for (size_t i = 1; i < b.params.size(); i++) {
      printf(", %s", b.params[i].name.c_str());
    }
  }
  printf(")\nBehavior body: ");
  ptree(b.root);
  printf("\n");
}

void print_error(std::ifstream& is, Err const& err) {
  is.seekg(err.posb.off, is.beg);
  size_t len = err.pose.off - err.posb.off + 1;
  char* buf = new char[len];
  is.read(buf, len - 1);
  buf[len - 1] = 0;
  printf("Failure at line %d column %d:\n  %s\n%s\n", err.posb.line, err.posb.col, buf, err.msg.c_str());
}

int main() {
  std::ifstream is("sample.blc");
  Result<ParsedScript> res = lang::parse_input(is);
  if (res.has_value()) {
    ParsedScript s = std::move(res.val());
    for (auto const& [k, v] : s.behaviors) {
      print_behavior(v);
    }
  } else {
    print_error(is, res.err());
  }
  return 0;
}
