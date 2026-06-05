// vogls: verify-stdout
// vogls: annotate-sdf
`timescale 1fs / 1fs
module buffer(input a, output b);
    assign b = a;
endmodule
module top();
    reg a;
    wire b;

    buffer _b(a, b);
    always #0 begin
        $display("[T=%0dns] b = %b", $time / 1_000_000, b);
        @(b);
    end
    initial begin
        #1;
        a = 1;
    end
endmodule
