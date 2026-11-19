// vogls: verify-diagnostics
// vogls: backend=bytecode
// vogls: mode=four-value-logic
// vogls: fail=lower
module m(inout wand p);
    assign p = 1'b0;
endmodule

module tb();
    reg a;
    wire w;
    assign w = a;
    m inst(w);
endmodule
