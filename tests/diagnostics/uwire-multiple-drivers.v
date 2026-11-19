// vogls: verify-diagnostics
// vogls: backend=bytecode
// vogls: mode=four-value-logic
// vogls: fail=lower
module tb();
    reg a, b;
    uwire [1:0] u;
    assign u = {a, b};
    assign u[1] = b;
endmodule
