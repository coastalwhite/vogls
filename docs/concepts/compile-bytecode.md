# Compile vs. Bytecode

Vogls transforms Verilog into its intermediate representation called the _Vogls Intermediate Representation_ (VIR). This representation can then be converted into a format that is suited for bytecode evaluation (default) or native code (using the `-C`) flag. The following is a pros and cons table.

| Property | Description | Bytecode | Compile |
|----------|-------------|----------|---------|
| Preparation time | Time to turn Verilog into format used for execution | Fast | Slow |
| Simulation time | Time to execute simulation instructions | Slow | Fast |
| Portability | Ability to run in every environment (e.g. WebAssembly) | Yes | No |
