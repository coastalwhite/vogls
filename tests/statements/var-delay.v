// vogls: verify-stdout
`timescale 1ps / 1ps
module tb();
    localparam PERIOD = 100_000;
    initial begin
        $display("T=%0d", $time());
        #(PERIOD / 2);
        $display("T=%0d", $time());
        #(PERIOD / 4);
        $display("T=%0d", $time());
    end
endmodule