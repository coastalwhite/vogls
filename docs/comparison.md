# Comparison to other tools

Vogls simulation is Verilog compliant but tunable to specific use-cases.

By default, Vogls interprets your design, but it can also compile your Verilog to native code. See [Compile vs. Bytecode](./concepts/compile-bytecode.md) for more information.

By default, Vogls uses two-value logic for wires and registers, but it can also use four-value logic. See [Two-value vs. Four-value logic](./concepts/two-four-value-logic.md) for more information.

## Icarus Verilog

Icarus Verilog is the closest to Vogls in terms of simulation model. It also implements Verilg simulation semantics, but does not provide a native code execution strategy and always uses four-value logic.

Icarus Verilog is more mature, better tested and likely what designs are tested against. However, it is relatively slow and difficult to embed. We use Icarus Verilog as a reference simulator for the Verilog semantics. However, do note that Verilog simulation semantics has race conditions, so Vogls and Icarus Verilog might not always agree.

## Verilator

Verilator compiles a subset of (System)Verilog to C++. This allows for very fast simulation performance, but creates long compilation times and may not be usable for full-timing simulations.

Verilator is generally a better option than Vogls if you mostly care about cycle-accurate simulation performance and don't need to use it in another tool.

[Verilog]: https://en.wikipedia.org/wiki/Verilog
