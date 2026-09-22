# REPORT SUMMARY
[FRONTEND](#frontend)



# FRONTEND
Developer logs while making Experimental Stage-3 .FYL compiler aka `ES3FL FRONTEND`

[REPORT-1](#report---1) \
[REPORT-1-RESULTS](#results) \
[REPORT-1-WALLS](#walls)
[REPORT-2-LEXER.FYL](#report---2-lexerfyl) \
[REPORT-2-RESULTS](#results-1) \
[REPORT-2-WALLS](#walls-1)


## REPORT - 1

### Results
---

* finished token.fyl
* finished source.fyl
* finished diagnostics.fyl

### Walls
---

> LEVEL: MEDIUM \
> STATUS: NOT-ADDED
1. Currently we dont have `using` statement meaning i had to import parts of the code fully instead of a small part that i needed.

## REPORT - 2 LEXER.FYL

### RESULTS
---

* Lexer finished
* Tokenization implemented
* Source spans implemented
* Comments implemented
* Literal escapes implemented
* Malformed lexical input detected
* Diagnostics implemented
* Also i ate food finnaly - This is important btw (Sorry for variable names i was hungry back then)
* Why am i even writing reports no one will read them, and if you are reading them - respect 

### WALLS
---

> LEVEL: MEDIUM, NOT-ADDED-YET \
> STATUS: NOT-ADDED
1. Unable to directly print enums and other complex data types, making debugging harder.

> LEVEL: LOW, NOT-ADDED-YET \
> STATUS: NOT-ADDED
2. Missing string interpolation. Diagnostic/debug output requires multiple print calls or manual string construction.

