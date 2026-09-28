// vogls: fail=lower
`default_nettype none

module sink (o, i);
  output o;
  input  i;
  assign o = i;
endmodule

module tb;
  wire c;

  sink m1 (.o(c), .i(i0));
endmodule
