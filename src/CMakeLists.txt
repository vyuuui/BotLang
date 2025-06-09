add_library(compiler-lib
  lang/compile.hh
  lang/result.hh
  lang/impl/ast.hh
  lang/impl/lex.cc
  lang/impl/lex.hh
  lang/impl/parse.hh
  lang/impl/parse.cc
  lang/impl/parsecomb.hh
  lang/impl/token.cc
  lang/impl/token.hh
  utl/queue.hh
  utl/resvec.hh
  utl/slaballoc.cc
  utl/slaballoc.hh
  utl/utf8.cc
  utl/utf8.hh
)

target_include_directories(compiler-lib PUBLIC ${PROJECT_SOURCE_DIR}/src)
