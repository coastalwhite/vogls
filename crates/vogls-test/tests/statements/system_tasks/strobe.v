`timescale 1ns/1ps
module strobe_radix;
 
  reg [11:0] r12;
  reg  [7:0] a;
  reg  [7:0] b;
 
  initial begin
    r12 = 12'h0a5;
    a   = 8'h01;
    b   = 8'h00;
 
    $display("0 display a=%0h b=%0h", a, b);
    $strobe ("0 strobe  a=%0h b=%0h", a, b);
 
    a  = 8'h02;
    b <= 8'h03;
 
    $display("0 display a=%0h b=%0h", a, b);
 
    #1;
    $strobe("1 strobe  a=%0h b=%0h", a, b);
    $strobe("1 second strobe");
    a = 8'h04;
 
    #1;
    $strobeb(r12);
    $strobeo(r12);
    $strobeh(r12);
    $strobe ("2 strobe  r12=%0h", r12);
 
    r12 = 12'h1f0;
 
    #1;
    $strobe;
  end
endmodule
