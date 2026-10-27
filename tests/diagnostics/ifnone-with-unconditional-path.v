// vogls: verify-diagnostics
// vogls: backend=bytecode
// vogls: mode=two-value-logic
// vogls: fail=lower
module gate(input a, output out);
    assign out = a;

    specify
        ifnone (a => out) = (1, 1);
        (a => out) = (1, 1);
    endspecify
endmodule
