// vogls: verify-stdout
`timescale 1ns/1ps
module tb;
  initial begin
    #2.375;
    $timeformat(-9,  0, " ns", 1);  $display("[%t]", $realtime);
    $timeformat(-9,  0, " ns", 10); $display("[%t]", $realtime);
    $timeformat(-9,  3, " ns", 10); $display("[%t]", $realtime);
    $timeformat(-12, 0, " ps", 12); $display("[%t]", $realtime);
    $timeformat(-6,  6, " us", 15); $display("[%t]", $realtime);
    $finish;
  end
endmodule
