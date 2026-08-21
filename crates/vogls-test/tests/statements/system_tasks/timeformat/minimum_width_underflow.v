// vogls: verify-stdout
`timescale 1ns/1ns
module tb;
  initial begin
    #123;
    $timeformat(-9, 2, " ns",  0); $display("[%t]", $realtime);
    $timeformat(-9, 2, " ns",  3); $display("[%t]", $realtime);
    $timeformat(-9, 2, " ns", 20); $display("[%t]", $realtime);
    $finish;
  end
endmodule
