# Grammar

This is a partial grammar for forms accepted by the bootstrap. It does not define unspecified language areas. Literal token definitions are in [lexical.md](lexical.md).

```text
module          = { import-decl | struct-decl | enum-decl | function-decl } ;
import-decl     = "use" module-name ";" ;
module-name     = identifier { "." identifier } ;
struct-decl     = "struct" identifier "{" { type identifier ";" } "}" ;
enum-decl       = "enum" identifier "{" identifier { "," identifier } [ "," ] "}" ;
function-decl   = visibility type identifier "(" [ parameters ] ")" block ;
visibility      = "public" | "private" ;
parameters      = parameter { "," parameter } ;
parameter       = type identifier ;

type            = basic-type
                | identifier
                | "list" type
                | type "[" integer-literal "]"
                | "string" "[" "]"
                | "(" type { "," type } ")" ;
basic-type      = "int" | "byte" | "float" | "bool" | "char"
                | "string" | "void" | "dynamic" ;

block           = "{" { statement } "}" ;
statement       = type identifier "=" expression ";"
                | assignable "=" expression ";"
                | "return" [ expression ] ";"
                | expression ";"
                | "if" "(" expression ")" block [ "else" block ]
                | "while" "(" expression ")" block
                | "break" ";"
                | "continue" ";" ;

expression      = logical-or ;
logical-or      = logical-and { "||" logical-and } ;
logical-and     = equality { "&&" equality } ;
equality        = comparison { ( "==" | "!=" ) comparison } ;
comparison      = additive { ( "<" | "<=" | ">" | ">=" ) additive } ;
additive        = multiplicative { ( "+" | "-" ) multiplicative } ;
multiplicative  = unary { ( "*" | "/" ) unary } ;
unary           = ( "-" | "!" ) unary | postfix ;
postfix         = primary { call | member | index } ;
call            = "(" [ arguments ] ")" ;
member          = "." identifier [ "(" [ arguments ] ")" ] ;
index           = "[" expression "]" ;
arguments       = expression { "," expression } ;
assignable      = identifier { member | index } ;
```

Primary expressions currently include identifiers, known literals, parenthesized expressions, collection literals, and named struct literals. Qualified enum variants use `EnumName.variant`.

`int(expression)` and `char(expression)` use ordinary call-shaped syntax but
are checked as explicit conversions rather than function calls.

Tuple return types are parsed, but tuple value syntax is not specified yet. `using` remains reserved and has no grammar. An `else if` shorthand is not currently accepted; nested `if` inside `else` can express the same control shape.
