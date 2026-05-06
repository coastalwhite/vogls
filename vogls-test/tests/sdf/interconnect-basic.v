// vogls: verify-stdout
// vogls: annotate-sdf
module buffer(input a, output b);
    assign b = a;
endmodule
module top();
    reg in;
    wire mid, out;

    buffer u1 (.a(in),  .b(mid));
    buffer u2 (.a(mid), .b(out));

    always @(out) $display("[T=%0dns] out = %b", $time / 1_000_000, out);
    initial begin
        #0;
        in = 1;
    end
endmodule
