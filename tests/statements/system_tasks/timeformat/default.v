// vogls: verify-stdout
`timescale 1ns/1ps
module tb;
  initial begin
    #3 $display("[%t]", $realtime);
    $finish;
  end
endmodule
