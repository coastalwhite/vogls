// vogls: verify-stdout
`timescale 1ns/1ps
module monitor_radix;
 
  reg [11:0] r12;
  reg  [7:0] a;
  reg  [7:0] b;
 
  initial begin
    a   = 8'h00;
    b   = 8'h00;
    r12 = 12'h0a5;
 
    $monitor("mon t=%0d a=%0h b=%0h", $time, a, b);
 
    #1 a = 8'h01;
    #1 a = 8'h02; b = 8'h02;
 
    #1 r12 = 12'h1f0;
    #1 $monitoroff;
       a = 8'h03;
 
    #1 $monitoron;
    #1 $display("d t=%0d b=%0h", $time, b);
       b <= 8'h04;
 
    #1 $monitor("new t=%0d a=%0h", $time, a);
 
    #1 b = 8'h05;
 
    #1 a = 8'h06;
 
    #1 $monitorh(r12);
    #1 r12 = 12'h00c;
    #1 $monitorb(r12);
    #1 r12 = 12'h1a0;
    #1 $monitoro(r12);
    #1 r12 = 12'h1f0;
 
    #1;
  end
endmodule
