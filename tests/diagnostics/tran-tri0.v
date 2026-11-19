// vogls: verify-diagnostics
// vogls: backend=bytecode
// vogls: mode=four-value-logic
// vogls: fail=lower
module tb();
    reg a;
    tri0 t;
    wire w;
    assign w = a;
    tran (t, w);
endmodule
