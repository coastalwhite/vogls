Optimization
- [x] IR text format
- [ ] Benchmarks
- [ ] Separate VM stackrefs into persistent and non-persistent.
      Take the max of all live non-persistant stackrefs as the size of the
      non-persistent memory size
  - IR
    - [ ] Common Subexpression Elimination
    - [x] Constant Propagation
    - [x] Deadcode elimination
    - [ ] Branches to lookup tables
    - [ ] Lookup Table Instruction
    - [ ] Lookup Table Optimization
    - [ ] Select Instruction
    - [ ] Select Inference
    - [ ] Peephole optimization
      - [ ] Concat -> Slice / Slice -> Concat
      - [ ] Multiply to Shift
    - [ ] Variable length concat
    - [ ] Loop unrolling
  - Flow Graph
    - [ ] Fuse Always 
    - [ ] Internalize signals into process, when only used there
    - [ ] Inline single watch, single drive processes
    - [ ] Simplify from predicates e.g. `if (i < 8) x[i]`.
    - [ ] Simplify signals from FV to TV where possible
    - [ ] Simplify CFG
      - [ ] Remove redundant jumps
      - [ ] Remove redundant branches

Long Term Goals
- [ ] Web Playground
- [ ] LLVM Backend
- Language Support
  - [ ] SystemVerilog
  - [ ] VHDL
  - [ ] FIRRTL
- [ ] Self-Hosted Assembly Backend
- [ ] Self-Hosted WebAssembly Backend
- [ ] Partially Recoverable Parsing
- [ ] Coverage Information