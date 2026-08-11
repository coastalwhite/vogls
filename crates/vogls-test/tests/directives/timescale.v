// vogls: verify-stdout
`timescale 1fs / 1fs
module top();
    a a();
    b b();
endmodule

`timescale 1ns / 1ns
module a();
    initial #1 $display("T = %0d", $time()); 
endmodule

`timescale 1ps / 1ns
module b();
    initial #2 $display("T = %0d", $time()); 
endmodule
