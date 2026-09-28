// vogls: verify-stdout
// vogls: timeout=100ps
`timescale 1ps / 1ps
module x();
    initial forever #17 $display("Hello!");
endmodule
