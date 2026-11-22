// vogls: verify-stdout
// The delays of net assignments and gates are in the time unit of their own module, also when a
// module with another timescale is lowered after it.
`timescale 1ns/1ps
module top;
    reg a = 0;
    wire y, g;
    assign #1 y = a;
    buf #(1, 2) b(g, a);
    sub s();
    initial begin
        #2 a = 1;
        #0.5 $display("%t y=%b g=%b", $realtime, y, g);
        #1 $display("%t y=%b g=%b", $realtime, y, g);
        a = 0;
        #1.5 $display("%t y=%b g=%b", $realtime, y, g);
        #1 $display("%t y=%b g=%b", $realtime, y, g);
    end
endmodule

`timescale 1ps/1ps
module sub;
endmodule
