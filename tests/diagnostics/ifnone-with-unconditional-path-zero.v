// vogls: verify-diagnostics
// vogls: backend=bytecode
// vogls: mode=two-value-logic
// vogls: fail=lower
// The paths have a delay of zero, which still is an error.
module gate(input a, output out);
    assign out = a;

    specify
        ifnone (a => out) = 0;
        (a => out) = 0;
    endspecify
endmodule
