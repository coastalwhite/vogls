// vogls: verify-stdout
`timescale 1ps/1ps
module tb;
  initial begin
    #1234567;
    $timeformat(-9, 0, " ns", 12); $display("[%t]", $realtime);
    $timeformat(-9, 2, " ns", 12); $display("[%t]", $realtime);
    $timeformat(-9, 3, " ns", 12); $display("[%t]", $realtime);
    $timeformat(-9, 6, " ns", 12); $display("[%t]", $realtime);
    $finish;
  end
endmodule
