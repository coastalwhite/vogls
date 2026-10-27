// vogls: verify-diagnostics
// vogls: backend=bytecode
// vogls: mode=two-value-logic
// vogls: fail=parse
// vogls: no-cadence-ifnone-edge
module gate(input a, output s);
    assign s = a;

    specify
        ifnone (posedge a => (s : a)) = 1;
    endspecify
endmodule
