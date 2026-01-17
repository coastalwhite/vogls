Verilog
- [ ] `$monitor`
- [ ] Allow `function` in constant contexts
- [ ] Make distinction between `wire` and `reg`
- [ ] Hierarchical Module Identifiers

- [ ] Four Value Logic
  - [ ] Separate instructions into `fv.*`
  - [ ] Add kernels for `fv.*`.

Optimization
- [ ] IR text format
- [ ] Separate VM stackrefs into persistent and non-persistent.
      Take the max of all live non-persistant stackrefs as the size of the
      non-persistent memory size.
- IR
  - [ ] Common Subexpression Elimination
  - [ ] Constant Propagation
  - [ ] Branches to lookup tables
- Flow Graph
  - [ ] Fuse Always 
  - [ ] Make signals into vars when only used once
  - [ ] Inline single watch, single drive processes

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