// vogls: verify-stdout
// vogls: annotate-sdf
module buffer(input a, output b);
    assign b = a;
endmodule
module top();
    reg a;
    wire b;

    buffer _b(a, b);
    always @(b) $display("[T=%0d] b = %b", $time, b);
    initial begin
        #0;
        a = 1;
    end
endmodule
