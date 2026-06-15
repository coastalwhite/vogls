// vogls: verify-ir
module top();
  reg [1:0] A;
  wire [1:0] B;
  
  BUF buf1( .in (A[0]), .out(B[0]) );
  BUF buf2( .in (A[1]), .out(B[1]) );
endmodule

module BUF(input in, output out);
  assign out = in;
endmodule
