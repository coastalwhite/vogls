// vogls: verify-diagnostics
// vogls: backend=bytecode
// vogls: mode=four-value-logic
// vogls: fail=lower
module tb();
    wire [1:0] v;
    wire s;
    tran t (v, s);
endmodule
