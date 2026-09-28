`timescale 1ms/1ms
module y();
    initial #5 $display("5ms %0d", $time);
endmodule

module z();
    y _y();
endmodule
