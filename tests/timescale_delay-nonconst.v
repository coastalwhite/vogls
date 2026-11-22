// vogls: verify-stdout
// Delays of net assignments and gates that are not constant are in the time unit of their own
// module, also when a module with another timescale is lowered after it.
`timescale 1ns/1ps
module top;
    reg a = 0;
    reg [7:0] d = 3;
    wire y, g;
    assign #(d) y = a;
    buf #(d) b(g, a);
    sub s();
    initial begin
        #10 a = 1;
        #1 $display("%t y=%b g=%b", $realtime, y, g);
        #2.5 $display("%t y=%b g=%b", $realtime, y, g);
        a = 0;
        #1 $display("%t y=%b g=%b", $realtime, y, g);
        #2.5 $display("%t y=%b g=%b", $realtime, y, g);
    end
endmodule

`timescale 1ps/1ps
module sub;
endmodule
