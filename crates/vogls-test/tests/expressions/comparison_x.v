module top;
  reg ca1;
  function [23:0] vf_func0;
    input [23:0] p0;
    vf_func0 = ((p0 / (&(24'h5ccee6 ** 2))) >= (((1'h0 >> 1'h1) ? (-p0) : (24'h82686b < 24'hc3a4da)) != (~(p0 != p0))));
  endfunction
  initial begin
    ca1 = (vf_func0(7'd3) <= 1'h1);
    $vogls_assert_eq(ca1, 1);
  end
endmodule
