#set document(title: "Verilog IEEE 1364-2005 Feature Support Checklist")
#set page(paper: "a4", margin: (x: 2cm, y: 2cm))
#set text(font: "New Computer Modern", size: 10pt)
#set heading(numbering: "1.1")
#show heading.where(level: 1): it => {
  v(1em)
  block(fill: luma(230), inset: (x: 6pt, y: 5pt), radius: 3pt, width: 100%)[
    #it
  ]
}
#show heading.where(level: 2): it => {
  v(0.5em)
  it
  v(0.2em)
}

#let done    = text(fill: rgb("#2e7d32"), weight: "bold")[$checkmark$]
#let missing = text(fill: rgb("#c62828"), weight: "bold")[$crossmark$]
#let partial = text(fill: rgb("#e65100"), weight: "bold")[$tilde$]

#let feature-table(..rows) = {
  table(
    columns: (1fr, 1.6cm, 1fr),
    align: (left, center, left),
    stroke: (x, y) => if y == 0 { (bottom: 0.8pt + black) } else { (bottom: 0.3pt + luma(200)) },
    fill: (x, y) => if y == 0 { luma(240) } else if calc.odd(y) { luma(250) } else { white },
    table.header(
      text(weight: "bold")[Feature],
      text(weight: "bold")[Status],
      text(weight: "bold")[Notes],
    ),
    ..rows
  )
}

#align(center)[
  #text(size: 18pt, weight: "bold")[Verilog IEEE 1364-2005 \ Feature Support Checklist]
  #v(0.4em)
  #text(size: 10pt, fill: luma(80))[#done Implemented #h(1em) #missing Not implemented #h(1em) #partial Partial]
]

#v(1em)

= Lexical Conventions (§2)

#feature-table(
  [Line comments (`//`)],                          done,    [],
  [Block comments (`/* */`)],                      done,    [],
  [Integer literals (decimal)],                    done,    [],
  [Integer literals (binary `'b`)],                done,    [],
  [Integer literals (octal `'o`)],                 done,    [],
  [Integer literals (hex `'h`)],                   done,    [],
  [Sized literals (`8'hFF`)],                      done,    [],
  [Unsized literals (`'b1010`)],                   done,    [],
  [X and Z digits in literals],                    done,    [],
  [Underscore separators in literals],             done,    [],
  [Real number literals (`3.14`)],                 missing, [Requires `real` type],
  [Real literals with exponent (`1.2e3`)],         missing, [Requires `real` type],
  [String literals (`"hello"`)],                   partial, [Non-standard internal type; not fully §4.1.10 compliant],
  [Escaped identifiers (`\name `)],                done,    [],
  [System identifiers (`$name`)],                  done,    [],
  [Compiler directives (#raw("`")-prefixed)],        partial, [See Compiler Directives section],
)

= Data Types & Declarations (§4, §6)

== Net types

#feature-table(
  [`wire`],                                        partial, [Parsed; multi-driver resolution distinct from `reg` incomplete (§4.1)],
  [`tri`],                                         missing, [Alias for `wire` with tri-state semantics],
  [`wand` / `triand`],                             missing, [Wired-AND resolution],
  [`wor` / `trior`],                               missing, [Wired-OR resolution],
  [`tri0`],                                        missing, [Pulls to 0 when undriven],
  [`tri1`],                                        missing, [Pulls to 1 when undriven],
  [`trireg`],                                      missing, [Retains last value when undriven; charge storage (§7.13.2)],
  [`supply0`],                                     missing, [Constant logic 0, supply strength],
  [`supply1`],                                     missing, [Constant logic 1, supply strength],
  [`uwire`],                                       missing, [Unresolved wire; error on multiple drivers],
  [Implicit wire declaration],                     missing, [Undeclared nets default to `wire` (§4.5)],
  [`default_nettype` control],                     missing, [Changes or disables implicit net type (§19.2)],
  [Net declarations with delay],                   missing, [`wire #5 w;` (§7.14)],
  [Net declarations with drive strength],          missing, [`wire (strong0, weak1) w;` (§6.1.4)],
  [Vectored / scalared modifiers],                 missing, [`vectored wire [7:0] v;` (§4.4)],
)

== Variable types

#feature-table(
  [`reg`],                                         done,    [],
  [`integer`],                                     done,    [],
  [`time`],                                        missing, [64-bit unsigned, simulation-time semantics (§4.1.3)],
  [`real`],                                        missing, [IEEE 754 double precision (§4.1.4)],
  [`realtime`],                                    missing, [Alias for `real` (§4.1.5)],
  [`event`],                                       missing, [Named synchronisation primitive; `-> ev` / `\@ev` (§4.1.6)],
)

== Parameters

#feature-table(
  [`parameter`],                                   done,    [],
  [`localparam`],                                  done,    [],
  [`specparam`],                                   partial, [Supported inside specify blocks; not all contexts],
  [Signed parameters],                             done,    [],
)

== Strengths (§6.1.4, §7.9–7.13)

#feature-table(
  [Supply strength (`supply0` / `supply1`)],       missing, [],
  [Strong strength],                               missing, [Default for most gates],
  [Pull strength],                                 missing, [],
  [Weak strength],                                 missing, [],
  [High-impedance (`highz0` / `highz1`)],          missing, [],
  [Multi-driver strength resolution],              missing, [Full resolution tables (§7.10)],
  [Strength reduction (resistive / non-resistive)],missing, [(§7.11, §7.12)],
)

= Expressions & Operators (§5)

#feature-table(
  [Addition `+`],                                  done,    [],
  [Subtraction `-`],                               done,    [],
  [Multiplication `*`],                            done,    [],
  [Division `/`],                                  done,    [],
  [Modulus `%`],                                   done,    [],
  [Power `**`],                                    done,    [],
  [Unary minus `-`],                               done,    [],
  [Unary plus `+`],                                done,    [],
  [`<`, `>`, `<=`, `>=`],                          done,    [],
  [`==`, `!=` (4-value, X-propagating)],           done,    [],
  [`===`, `!==` (case equality, X/Z exact)],       done,    [],
  [`&&`, `||`, `!`],                               done,    [],
  [`&`, `|`, `^`, `~^` / `^~`, `~`],               done,    [],
  [Reduction `&`, `~&`, `|`, `~|`, `^`, `~^`],     done,    [Unary prefix form],
  [`<<`, `>>` (logical)],                          done,    [],
  [`<<<`, `>>>` (arithmetic)],                     done,    [],
  [Conditional `?:`],                              done,    [],
  [Concatenation `{a, b}`],                        done,    [],
  [Replication `{N{expr}}`],                       done,    [],
  [Bit-select `a[i]`],                             done,    [],
  [Part-select `a[hi:lo]`],                        done,    [],
  [Indexed part-select `a[base+:w]`, `a[base-:w]`],done,    [(§5.2.1)],
  [Signed / unsigned cast],                        done,    [`$signed`, `$unsigned`],
  [Real number expressions],                       missing, [Requires `real` type],
  [Mixed real / integer expressions],              missing, [(§5.5.1)],
)

= Assignments (§6.1, §9.2, §9.3)

#feature-table(
  [Continuous assignment (`assign`)],              done,           [],
  [Continuous assignment delay (`assign #5`)],     missing,        [(§6.1.3)],
  [Continuous assignment strength],                missing,        [(§6.1.4)],
  [Blocking procedural assignment (`=`)],          done,           [],
  [Non-blocking procedural assignment (`<=`)],     done,           [],
  [Intra-assignment delay (`= #5 expr`)],          missing,        [(§9.7.7)],
  [Intra-assignment event control (`= \@(clk) expr`)], missing,    [(§9.7.7)],
  [Intra-assignment repeat (`= repeat(N) \@(clk) expr`)], missing, [(§9.7.7)],
  [Procedural continuous `assign` / `deassign`],  missing,         [Overrides reg; released by `deassign` (§9.3.1)],
  [Procedural `force` / `release`],               missing,         [Overrides nets and regs; highest priority (§9.3.2)],
  [Assignment to bit-select of reg],              done,            [],
  [Assignment to part-select of reg],             done,            [],
  [Assignment to memory word],                    done,            [],
  [Concatenation as LHS],                         done,            [],
)

= Gate & Switch Level Modeling (§7)

== Basic logic gates

#feature-table(
  [`and`, `nand`],                                 done, [(§7.2)],
  [`or`, `nor`],                                   done, [(§7.2)],
  [`xor`, `xnor`],                                 done, [(§7.2)],
  [`buf`, `not`],                                  missing, [(§7.3)],
  [Multiple outputs on `buf` / `not`],             missing, [(§7.3)],
  [Gate arrays (`and a[3:0] (...)`)],              missing, [(§7.1.5)],
  [Gate delays (1, 2, or 3 values)],               missing, [(§7.14)],
  [Gate drive strength],                           missing, [(§7.1.2)],
)

== Tristate gates

#feature-table(
  [`bufif0`, `bufif1`],                            missing, [Output is `z` when control inactive (§7.4)],
  [`notif0`, `notif1`],                            missing, [Inverted tristate (§7.4)],
)

== MOS & CMOS switches

#feature-table(
  [`nmos`, `pmos`],                                missing, [(§7.5)],
  [`rnmos`, `rpmos` (resistive)],                  missing, [(§7.5)],
  [`cmos`, `rcmos`],                               missing, [(§7.7)],
)

== Bidirectional pass switches

#feature-table(
  [`tran`, `tranif0`, `tranif1`],                  missing, [(§7.6)],
  [`rtran`, `rtranif0`, `rtranif1` (resistive)],   missing, [(§7.6)],
)

== Pull sources

#feature-table(
  [`pullup`, `pulldown`],                          missing, [(§7.8)],
)

= User-Defined Primitives (§8)

#feature-table(
  [Combinational UDPs],                            done,    [(§8.2)],
  [Level-sensitive sequential UDPs],               done,    [(§8.3)],
  [Edge-sensitive sequential UDPs],                done,    [(§8.4)],
  [Sequential UDP initialisation],                 done,    [`initial output = 1'b0;` (§8.5)],
  [UDP instantiation with delay],                  partial, [Gate-level delay infrastructure incomplete],
  [UDP instantiation with drive strength],         missing, [(§8.6)],
  [Mixed level / edge-sensitive tables],           missing, [(§8.7)],
  [Level-sensitive dominance],                     missing, [(§8.8)],
  [Z values in UDP tables],                        missing, [(§8.1.5)],
)

= Behavioral Modeling (§9)

== Structured procedures

#feature-table(
  [`initial`],                                     done,    [(§9.9.1)],
  [`always`],                                      done,    [(§9.9.2)],
)

== Procedural statements

#feature-table(
  [`if` / `else`],                                 done,    [(§9.4)],
  [`if`-`else if` chains],                         done,    [(§9.4.1)],
  [`case`],                                        done,    [(§9.5)],
  [`casex` (X/Z as don't-care)],                   done,    [(§9.5.1)],
  [`casez` (Z as don't-care)],                     done,    [(§9.5.1)],
  [`case` with constant expression],               done,    [(§9.5.2)],
  [`for` loop],                                    done,    [(§9.6)],
  [`while` loop],                                  done,    [(§9.6)],
  [`repeat` loop],                                 done,    [(§9.6)],
  [`forever` loop],                                done,    [(§9.6)],
  [`disable` (named block / task)],                done,    [(§10.3)],
  [`wait` (level-sensitive event)],                done,    [(§9.7.6)],
)

== Timing controls

#feature-table(
  [Delay control (`#N`)],                          done,    [(§9.7.1)],
  [Delay with expression (`#(expr)`)],             done,    [],
  [Delay min:typ:max (`#(1:2:3)`)],                done,    [Always uses typical.],
  [Event control (`\@(signal)`)],                  done,    [(§9.7.2)],
  [`posedge` / `negedge` events],                  done,    [(§9.7.2)],
  [Event OR (`\@(a or b)` / `\@(a, b)`)],          done,    [(§9.7.4)],
  [Implicit event list (`\@*` / `\@(*)`)],         done,    [(§9.7.5)],
  [Named events (`event ev; -> ev; \@ev`)],        missing, [Requires `event` type],
)

== Block statements

#feature-table(
  [Sequential blocks (`begin` / `end`)],           done,    [(§9.8.1)],
  [Parallel blocks (`fork` / `join`)],             done,    [(§9.8.2)],
  [Named blocks],                                  done,    [(§9.8.3)],
  [Local variables in named blocks],               done,    [],
)

= Tasks & Functions (§10)

#feature-table(
  [Task declaration],                              partial, [(§10.2.1)],
  [Task enabling (call)],                          partial, [(§10.2.2)],
  [Task input / output / inout ports],             partial, [Copy-in / copy-out semantics],
  [Non-automatic task (static storage)],           missing, [Default; shared state across calls (§10.2.3)],
  [`automatic` task (dynamic storage)],            partial, [Stack frame per call; re-entrant (§10.2.3)],
  [Task with timing controls],                     partial, [Tasks may consume simulation time],
  [Function declaration],                          partial, [(§10.4.1)],
  [Recursive function],                            missing, [Not supported. Not planned to be supported],
  [Function call],                                 partial, [(§10.4.3)],
  [Function return value],                         partial, [Assigned to function name (§10.4.2)],
  [Non-automatic function (static)],               missing, [(§10.4.4)],
  [`automatic` function (recursive)],              partial, [(§10.4.4)],
  [Constant functions],                            partial, [Evaluated at elaboration time (§10.4.5)],
  [Functions calling other functions],             partial, [No tasks, no timing controls],
  [`disable` inside tasks],                        missing, [Early return / break (§10.3)],
)

= Hierarchical Structures (§12)

== Modules & instantiation

#feature-table(
  [Module definition],                             done,    [],
  [Module instantiation by port order],            done,    [],
  [Module instantiation by port name],             done,    [],
  [Unconnected ports (left blank)],                done,    [],
  [Top-level module detection],                    done,    [],
  [Nested module hierarchy],                       done,    [],
  [Module instance arrays],                        missing, [`my_mod inst [3:0] (...)` (§12.1.2)],
  [`defparam` statement],                          missing, [Cross-hierarchy parameter override (§12.2.1)],
  [Module instance parameter by order (`#(8, 1)`)],done,   [(§12.2.2)],
  [Module instance parameter by name (`#(.W(8))`)],done,   [(§12.2.2)],
)

== Ports (§12.3)

#feature-table(
  [`input`, `output`, `inout` ports],              done,    [],
  [Port connection rules (net vs. variable)],      partial, [Depends on wire/reg distinction completion],
  [Dissimilar port widths (implicit truncation)],  done,    [(§12.3.8)],
  [Signed values via ports],                       done,    [(§12.3.11)],
  [Real number port connections],                  missing, [(§12.3.7)],
)

== Generate constructs (§12.4)

#feature-table(
  [Loop generate (`genvar` + `for`)],              done,    [(§12.4.1)],
  [Conditional generate (`if` / `else`)],          done,    [(§12.4.2)],
  [`case` generate],                               done,    [(§12.4.2)],
  [Named generate blocks],                         done,    [(§12.4.3)],
)

== Scope & naming (§12.5–12.8)

#feature-table(
  [Hierarchical names (`top.u1.sig`)],             done,    [(§12.5)],
  [Upward name referencing],                       done,    [(§12.6)],
  [Scope rules],                                   done,    [(§12.7)],
  [Elaboration order],                             done,    [(§12.8)],
)

== Library & configuration (§13)

#feature-table(
  [Library map files (`.map`)],                      missing, [(§13.2.1)],
  [`config` / `endconfig` blocks],                   missing, [(§13.3)],
  [`design`, `instance`, `cell`, `default` clauses], missing, [(§13.3.1)],
)

= Specify Blocks (§14)

== Module path declarations

#feature-table(
  [Specify block syntax (`specify` / `endspecify`)], done,  [(§14.1)],
  [Simple path (`(a => b) = 5`)],                    done,    [(§14.2.2)],
  [Full connection path (`(a, b *> c, d) = 5`)],     missing, [n×m parallel (§14.2.5)],
  [Parallel connection path (`(a, b => c, d) = 5`)], missing, [(§14.2.5)],
  [Multiple paths in one statement],                 missing, [(§14.2.6)],
  [Path polarity operator],                          missing, [`+:` / `-:` polarity (§14.2.7)],
  [Edge-sensitive path],                             done,    [`(posedge clk => (q:d)) = 5` (§14.2.3)],
  [State-dependent path (`if`)],                     done,    [(§14.2.4)],
  [State-dependent path (`ifnone`)],                 missing, [Else-branch fallback (§14.2.4)],
  [`specparam` declarations],                        missing, [],
)

== Path delays

#feature-table(
  [1-value delay],                                 done,    [],
  [2-value delay (rise, fall)],                    done,    [],
  [3-value delay (rise, fall, turn-off)],          done,    [],
  [6-value delay (all transition pairs)],          done,    [],
  [12-value delay],                                done,    [],
  [X-transition delays],                           partial, [(§14.3.2)],
  [min:typ:max on path delays],                    partial, [(§14.3.3)],
  [Mixing path and distributed delays],            missing, [(§14.4)],
)

== Pulse filtering (§14.6)

#feature-table(
  [Default inertial pulse rejection],              missing, [],
  [`PATHPULSE$` specparam],                        missing, [Per-path reject/error limits (§14.6.1)],
  [Global pulse limit invocation options],         missing, [(§14.6.2)],
  [`pulsestyle_onevent` declaration],              missing, [(§14.6.4)],
  [`pulsestyle_ondetect` declaration],             missing, [(§14.6.4)],
  [`showcancelled` / `noshowcancelled`],           missing, [(§14.6.4)],
)

= Timing Checks (§15)

#feature-table(
  [`$setup`],                                      missing, [(§15.2.1)],
  [`$hold`],                                       missing, [(§15.2.2)],
  [`$setuphold`],                                  missing, [(§15.2.3)],
  [`$recovery`],                                   missing, [(§15.2.5)],
  [`$removal`],                                    missing, [(§15.2.4)],
  [`$recrem`],                                     missing, [(§15.2.6)],
  [`$skew`],                                       missing, [(§15.3.1)],
  [`$timeskew`],                                   missing, [(§15.3.2)],
  [`$fullskew`],                                   missing, [(§15.3.3)],
  [`$width`],                                      missing, [(§15.3.4)],
  [`$period`],                                     missing, [(§15.3.5)],
  [`$nochange`],                                   missing, [(§15.3.6)],
  [Edge-control specifiers in timing check args],  missing, [(§15.4)],
  [Notifier registers],                            missing, [User-defined violation response (§15.5)],
  [Conditioned events (`&&& condition`)],          missing, [(§15.6)],
  [Vector signals in timing checks],               missing, [(§15.7)],
  [Negative timing checks],                        missing, [(§15.8)],
)

= SDF Backannotation (§16)

#feature-table(
  [`$sdf_annotate` task],                          missing, [(§17.2.10)],
  [SDF delay annotation to specify blocks],        missing, [(§16.2.1)],
  [SDF timing check annotation],                   missing, [(§16.2.2)],
  [SDF specparam annotation],                      missing, [(§16.2.3)],
  [SDF interconnect delay annotation],             missing, [(§16.2.4)],
  [Multiple SDF files],                            missing, [(§16.4)],
  [Pulse limit annotation via SDF],                missing, [(§16.5)],
)

= System Tasks — Display (§17.1)

== Format specifiers

#feature-table(
  [`%h` / `%H` (hex)],                             done,    [Also `%x` / `%X`],
  [`%d` / `%D` (decimal)],                         done,    [],
  [`%o` / `%O` (octal)],                           done,    [],
  [`%b` / `%B` (binary)],                          done,    [],
  [`%c` / `%C` (ASCII character)],                 missing, [Least-significant byte of value],
  [`%s` / `%S` (string)],                          missing, [Right-justified ASCII],
  [`%m` / `%M` (hierarchical scope name)],         missing, [Consumes no argument],
  [`%t` / `%T` (time, via `$timeformat`)],         missing, [],
  [`%v` / `%V` (net strength + value)],            missing, [e.g. `St0`, `HiZ1`],
  [`%l` / `%L` (library cell binding)],            missing, [Rarely needed; can stub],
  [`%e` / `%E` (real, scientific notation)],       missing, [Requires `real`],
  [`%f` / `%F` (real, decimal notation)],          missing, [Requires `real`],
  [`%g` / `%G` (real, shorter of `%e`/`%f`)],     missing, [Requires `real`],
  [`%u` / `%U` (unformatted 2-value binary)],      missing, [Raw byte dump],
  [`%z` / `%Z` (unformatted 4-value binary)],      missing, [Raw byte dump],
  [Escape sequences (`\n`, `\t`, `\\`, `\"`)],     partial, [Common ones work; octal `\NNN` may be missing],
  [Field width specifier (`%0d`, `%8h`)],          partial, [Basic support; edge cases may be missing],
)

== Display tasks

#feature-table(
  [`$display`],                                    done,    [Prints with trailing newline],
  [`$displayb`],                                   missing, [Default radix binary],
  [`$displayh`],                                   missing, [Default radix hex],
  [`$displayo`],                                   missing, [Default radix octal],
  [`$write`],                                      done,    [Like `$display` but no newline (§17.1.1)],
  [`$writeb`, `$writeh`, `$writeo`],               missing, [],
  [`$strobe`],                                     missing, [Prints at end of time step (§17.1.2)],
  [`$strobeb`, `$strobeh`, `$strobeo`],            missing, [],
  [`$monitor`],                                    missing, [Prints on any argument change (§17.1.3)],
  [`$monitorb`, `$monitorh`, `$monitoro`],         missing, [],
  [`$monitoroff` / `$monitoron`],                  missing, [Suspend / resume monitoring],
)

= System Tasks — File I/O (§17.2)

#feature-table(
  [`$fopen(filename)`],                            missing, [Returns multi-channel descriptor (§17.2.1)],
  [`$fopen(filename, type)`],                      missing, [With mode string `"r"`, `"w"`, `"a"`, etc.],
  [`$fclose(fd)`],                                 missing, [(§17.2.1)],
  [`$fdisplay(fd, ...)`],                          missing, [(§17.2.2)],
  [`$fdisplayb`, `$fdisplayh`, `$fdisplayo`],      missing, [],
  [`$fwrite(fd, ...)`],                            missing, [(§17.2.2)],
  [`$fwriteb`, `$fwriteh`, `$fwriteo`],            missing, [],
  [`$fstrobe(fd, ...)`],                           missing, [(§17.2.2)],
  [`$fmonitor(fd, ...)`],                          missing, [(§17.2.2)],
  [`$sformat(str, fmt, ...)`],                     missing, [Format to string variable (§17.2.3)],
  [`$swrite`, `$swriteb`, `$swriteh`, `$swriteo`], missing, [],
  [`$fscanf(fd, fmt, ...)`],                       missing, [Formatted read from file (§17.2.4)],
  [`$sscanf(str, fmt, ...)`],                      missing, [Formatted read from string],
  [`$fgetc(fd)`],                                  missing, [Read single character (§17.2.4)],
  [`$ungetc(c, fd)`],                              missing, [Push character back (§17.2.4)],
  [`$fgets(str, fd)`],                             missing, [Read line (§17.2.4)],
  [`$fread(mem, fd)`],                             missing, [Binary read into memory (§17.2.4)],
  [`$fseek(fd, offset, origin)`],                  missing, [(§17.2.5)],
  [`$ftell(fd)`],                                  missing, [(§17.2.5)],
  [`$rewind(fd)`],                                 missing, [(§17.2.5)],
  [`$fflush(fd)`],                                 missing, [(§17.2.6)],
  [`$ferror(fd, str)`],                            missing, [(§17.2.7)],
  [`$feof(fd)`],                                   missing, [(§17.2.8)],
  [`$readmemb(file, mem)`],                        partial, [Load binary data into memory (§17.2.9)],
  [`$readmemh(file, mem)`],                        partial, [Load hex data into memory (§17.2.9)],
  [`$readmemb` / `$readmemh` with address range],  missing, [`$readmemh("f", m, start, end)`],
)

= System Tasks — Timescale, Simulation Control (§17.3–17.4)

#feature-table(
  [`$printtimescale`],                             missing, [(§17.3.1)],
  [`$timeformat(unit, prec, suffix, width)`],      missing, [Controls `%t` formatting (§17.3.2)],
  [`$finish`],                                     done,    [Terminate simulation (§17.4.1)],
  [`$finish(n)`],                                  missing, [With verbosity level 0/1/2],
  [`$stop`],                                       missing, [Halt and enter interactive mode (§17.4.2)],
)

= System Tasks — PLA Modeling (§17.5)

#feature-table(
  [`$async$and$array`],                            missing, [],
  [`$async$nand$array`],                           missing, [],
  [`$async$or$array`],                             missing, [],
  [`$async$nor$array`],                            missing, [],
  [`$async$and$plane`],                            missing, [],
  [`$async$nand$plane`],                           missing, [],
  [`$async$or$plane`],                             missing, [],
  [`$async$nor$plane`],                            missing, [],
  [`$sync$and$array`],                             missing, [],
  [`$sync$nand$array`],                            missing, [],
  [`$sync$or$array`],                              missing, [],
  [`$sync$nor$array`],                             missing, [],
  [`$sync$and$plane`],                             missing, [],
  [`$sync$nand$plane`],                            missing, [],
  [`$sync$or$plane`],                              missing, [],
  [`$sync$nor$plane`],                             missing, [],
)

= System Tasks — Stochastic Analysis (§17.6)

#feature-table(
  [`$q_initialize`],                               missing, [(§17.6.1)],
  [`$q_add`],                                      missing, [(§17.6.2)],
  [`$q_remove`],                                   missing, [(§17.6.3)],
  [`$q_full`],                                     missing, [(§17.6.4)],
  [`$q_exam`],                                     missing, [(§17.6.5)],
)

= System Functions — Time, Conversion & Math (§17.7–17.11)

== Simulation time (§17.7)

#feature-table(
  [`$time`],                                       done,    [Returns 64-bit integer time (§17.7.1)],
  [`$stime`],                                      missing, [Returns 32-bit integer, truncated (§17.7.2)],
  [`$realtime`],                                   missing, [Returns real-valued time (§17.7.3)],
)

== Conversion (§17.8)

#feature-table(
  [`$signed(expr)`],                               done,    [Cast to signed],
  [`$unsigned(expr)`],                             done,    [Cast to unsigned],
  [`$bitstoreal(expr)`],                           missing, [64-bit integer bits → IEEE 754 double],
  [`$realtobits(expr)`],                           missing, [IEEE 754 double → 64-bit integer bits],
  [`$itor(expr)`],                                 missing, [Integer → real],
  [`$rtoi(expr)`],                                 missing, [Real → integer (truncates toward zero)],
)

== Probabilistic distributions (§17.9)

#feature-table(
  [`$random`],                                     partial, [Stub implemented; seeding may not be standard-compliant (§17.9.1)],
  [`$dist_uniform`],                               missing, [(§17.9.2)],
  [`$dist_normal`],                                missing, [],
  [`$dist_exponential`],                           missing, [],
  [`$dist_poisson`],                               missing, [],
  [`$dist_chi_square`],                            missing, [],
  [`$dist_t`],                                     missing, [],
  [`$dist_erlang`],                                missing, [],
)

== Command line input (§17.10)

#feature-table(
  [`$test$plusargs(string)`],                      missing, [Check for `+argname` on command line (§17.10.1)],
  [`$value$plusargs(format, var)`],                missing, [Read `+argname=value` (§17.10.2)],
)

== Math functions (§17.11)

#feature-table(
  [`$clog2(n)`],                                   missing, [Integer ceiling log2; common in params (§17.11.1)],
  [`$ln(x)`],                                      missing, [(§17.11.2)],
  [`$log10(x)`],                                   missing, [],
  [`$exp(x)`],                                     missing, [],
  [`$sqrt(x)`],                                    missing, [],
  [`$pow(x, y)`],                                  missing, [],
  [`$floor(x)`],                                   missing, [],
  [`$ceil(x)`],                                    missing, [],
  [`$sin(x)`],                                     missing, [],
  [`$cos(x)`],                                     missing, [],
  [`$tan(x)`],                                     missing, [],
  [`$asin(x)`],                                    missing, [],
  [`$acos(x)`],                                    missing, [],
  [`$atan(x)`],                                    missing, [],
  [`$atan2(y, x)`],                                missing, [],
  [`$hypot(x, y)`],                                missing, [],
  [`$sinh(x)`],                                    missing, [],
  [`$cosh(x)`],                                    missing, [],
  [`$tanh(x)`],                                    missing, [],
  [`$asinh(x)`],                                   missing, [],
  [`$acosh(x)`],                                   missing, [],
  [`$atanh(x)`],                                   missing, [],
)

= Value Change Dump — VCD (§18)

== Four-state VCD (§18.1)

#feature-table(
  [`$dumpfile(filename)`],                         done,    [Name the dump file (§18.1.1)],
  [`$dumpvars` (dump all)],                        done,    [Level 0 = all variables (§18.1.2)],
  [`$dumpvars(level, scope)`],                     partial, [Level-selective dump],
  [`$dumpoff` / `$dumpon`],                        partial, [Pause / resume dumping (§18.1.3)],
  [`$dumpall`],                                    missing, [Checkpoint all current values (§18.1.4)],
  [`$dumplimit(size)`],                            missing, [Limit dump file size (§18.1.5)],
  [`$dumpflush`],                                  missing, [Flush dump buffer (§18.1.6)],
  [VCD header (`$timescale`, `$date`, `$version`)],partial, [(§18.2.1). Date missing],
  [VCD variable declarations (`$var ... $end`)],   done,    [With proper scope tags],
  [VCD `$scope` / `$upscope` hierarchy],           done,    [(§18.2.3)],
  [VCD scalar value changes (`0`, `1`, `x`, `z`)], done,    [],
  [VCD vector value changes (`bXXXX id`)],         done,    [],
  [VCD real value changes (`r3.14 id`)],           missing, [],
  [Compact ID code assignment],                    missing, [Printable ASCII `!` through `~`],
)

== Extended VCD (§18.3)

#feature-table(
  [`$dumpports`],                                  missing, [Port-strength VCD (§18.3.1)],
  [`$dumpportsoff` / `$dumpportson`],              missing, [],
  [`$dumpportsall`],                               missing, [],
  [`$dumpportslimit`],                             missing, [],
  [`$dumpportsflush`],                             missing, [],
)

= Compiler Directives (§19)

#feature-table(
  [`define` (simple, no args)],               done,    [(§19.3.1)],
  [`define` (with parameters)],               partial, [e.g. `define MAX(a,b) ... ` (§19.3.1)],
  [`undef`],                                  done,    [(§19.3.2)],
  [`ifdef`],                                  done,    [(§19.4)],
  [`ifndef`],                                 done,    [(§19.4)],
  [`else`],                                   done,    [(§19.4)],
  [`elsif`],                                  done,    [(§19.4)],
  [`endif`],                                  done,    [(§19.4)],
  [`include`],                                done,    [(§19.5)],
  [`resetall`],                               missing, [Resets all directives to defaults (§19.6)],
  [`line`],                                   missing, [Source location override for diagnostics (§19.7)],
  [`timescale`],                              done,    [(§19.8)],
  [`default_nettype`],                        missing, [(§19.2)],
  [`celldefine` / `endcelldefine`],           missing, [Mark module as library cell; sim no-op (§19.1)],
  [`unconnected_drive`],                      missing, [Drive value on unconnected inputs (§19.9)],
  [`nounconnected_drive`],                    missing, [(§19.9)],
  [`pragma`],                                 missing, [Tool metadata; sim no-op (§19.10)],
  [`begin_keywords` / `end_keywords`],        missing, [Keyword set selection (§19.11)],
)

= Fast Signal Trace (FST)

#feature-table(
  [FST file writing],                              missing, [],
  [Hierarchical scope in FST],                     missing, [],
  [Compressed value change recording],             missing, [],
)

= Verilog Procedural Interface — VPI (§26)

#feature-table(
  [VPI callback registration (`vpi_register_cb`)], missing, [(§26.2.1)],
  [VPI handle traversal],                          missing, [(§26.2.2)],
  [Value read (`vpi_get_value`)],                  missing, [(§26.3.4)],
  [Value write (`vpi_put_value`)],                 missing, [(§26.3.4)],
  [Object property access (`vpi_get`)],            missing, [(§26.3.2)],
  [Delay access (`vpi_get_delays`)],               missing, [(§26.3.4)],
  [User-defined system tasks / functions via VPI], missing, [(§26.1)],
)

= Simulation Engine Semantics (§11)

#feature-table(
  [Active event region],                           done,    [],
  [NBA (non-blocking assignment update) region],   done,    [(§11.3)],
  [Inactive (`#0`) region],                        done,    [],
  [Monitor / postponed region],                    partial, [Required for `$monitor` / `$strobe` (§11.3)],
  [Future time step scheduling],                   done,    [],
  [Stratified event queue],                        done,    [(§11.3)],
  [Two-value logic mode],                          done,    [],
  [Four-value logic mode],                         done,    [],
)