module top;
  function [6:0] vf; input [16:0] p0; input [10:0] p1;
    begin vf = (p1 ? (7'h30 != p1) : (p0 < p1)); end
  endfunction
  reg [7:0] u0_y, u1_y; reg [8:0] r0;
  reg [12:0] a,d;
  initial begin
    u0_y=0; u1_y=12; r0=1;
    d = (vf(u1_y,r0) ? u0_y : u1_y);
    a = (1'h1 ? (vf(u1_y,r0) ? u0_y : u1_y) : 8'd7);
    $vogls_assert_eq(vf(u1_y,r0), 1);
    $vogls_assert_eq(d, 0);
    $vogls_assert_eq(a, 0);
  end
endmodule
