// vogls: verify-stdout
`timescale 1ns/1ps
module tf_args;
  initial begin
    $timeformat(-9, 3, " ns", 10);
    #2.6;
    $display("realtime [%t]", $realtime);
    $display("time     [%t]", $time);
    $display("literal  [%t]", 7);
    $finish;
  end
endmodule
