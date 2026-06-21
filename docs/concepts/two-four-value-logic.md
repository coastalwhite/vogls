# Two-value vs. Four-value logic

Traditional Verilog uses [four-value logic] which, apart from logical low `0` and logical high `1`, includes unknown `X` and high-impedance `Z` states. Certain Verilog designs rely on this for simulation, but most designs can run without these two extra states. Two value logic can be a lot faster and less memory consuming. Therefore, Vogls, by default, utilizes two-value logic for nets and registers. This means that for two-value logic mode:

- Each net or register gets initialized to `0` (instead of `X`).
- When a net or register is assigned to `X` or `Z`, it gets converted to a `0`.

Note that even in two-value logic mode, `X` and `Z` can still exist in intermediate variables. This allows Verilog constructs like `casex` to still work.

If you want full four-value logic for nets and registers, you can use the `-F` flag.

[four-value logic]: https://en.wikipedia.org/wiki/Four-valued_logic
