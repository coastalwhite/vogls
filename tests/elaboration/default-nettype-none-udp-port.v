// vogls: fail=lower
`default_nettype none

primitive udp_buf (o, i);
  output o;
  input  i;
  table
    0 : 0;
    1 : 1;
  endtable
endprimitive

module tb;
  wire c;

  udp_buf u1 (c, i0);
endmodule
