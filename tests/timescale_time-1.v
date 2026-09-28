`timescale 1fs/1fs
module x();
    initial #5 $display("5fs %0d", $time);
endmodule

`timescale 1ms/1ms
module y();
    initial #5 $display("5ms %0d", $time);
endmodule

module z();
    x _x();
    y _y();
endmodule
