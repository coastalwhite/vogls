// vogls: verify-diagnostics
// vogls: backend=bytecode
// vogls: mode=two-value-logic
// vogls: fail=parse
// vogls: no-data-sourceless-edge-path
module gate(input a, output s);
    assign s = a;

    specify
        (posedge a => s) = 1;
    endspecify
endmodule
