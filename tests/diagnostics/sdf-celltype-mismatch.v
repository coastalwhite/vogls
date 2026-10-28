// vogls: verify-diagnostics
// vogls: annotate-sdf
// vogls: backend=bytecode
// vogls: mode=two-value-logic
// vogls: fail=lower
module buffer(input a, output b);
    assign b = a;
endmodule
module wrap(input a, output b);
    buffer u(.a(a), .b(b));
endmodule
module top();
    reg a;
    wire b;

    wrap w(.a(a), .b(b));
endmodule
