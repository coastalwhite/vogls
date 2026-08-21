// vogls: verify-stdout
`timescale 1ns/1ps
module tb_ns;
  initial begin
    $timeformat(-9, 2, " ns", 8);
    #10 $display("ns module: [%t]", $realtime);
  end
endmodule

`timescale 1us/1ns
module tb_us;
  initial begin
    #10 $display("us module: [%t]", $realtime);
    $finish;
  end
endmodule

module tb;
    tb_ns x();
    tb_us y();
endmodule
