// vogls: verify-diagnostics
// vogls: annotate-sdf
// vogls: sdf-scope=tb.missing
// vogls: backend=bytecode
// vogls: mode=two-value-logic
// vogls: fail=lower
module buffer(input a, output b);
    assign b = a;
endmodule
module tb();
    reg a;
    wire b;

    buffer dut(.a(a), .b(b));
endmodule
