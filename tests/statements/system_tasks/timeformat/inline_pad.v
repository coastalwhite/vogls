`timescale 1ns/1ps
module tf_inline_width;
  initial begin
    #12.5;

    $timeformat(-9, 2, " ns", 15);
    $display("A default : [%t]",    $realtime);
    $display("A zero    : [%0t]",   $realtime);
    $display("A narrow  : [%3t]",   $realtime);
    $display("A exact   : [%8t]",   $realtime);
    $display("A wide    : [%20t]",  $realtime);

    $timeformat(-9, 2, " ns", 0);
    $display("B zerocfg : [%t]",    $realtime);
    $display("B inline  : [%12t]",  $realtime);

    $timeformat(-9, 2, " nanoseconds", 25);
    $display("C suffix  : [%t]",    $realtime);
    $display("C tooshort: [%10t]",  $realtime);
    $display("C zero    : [%0t]",   $realtime);

    $timeformat(-12, 1, "", 12);
    $display("D nosuffix: [%t]",    $realtime);
    $display("D zero    : [%0t]",   $realtime);
    $finish;
  end
endmodule
