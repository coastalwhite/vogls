// vogls: verify-stdout
`timescale 1ps / 1fs
module tb();
    initial begin
        $display("hi!");
        fork
            #20 $display("Time = %0d", $time);
            #10 $display("Time = %0d", $time);
        join
        $display("bye!");
    end
endmodule
