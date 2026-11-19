// vogls: mode=four-value-logic
//
// The state of a sequential UDP drives its output. When the output has no other driver, the
// output is the state. Otherwise the state is resolved with the other drivers.
//
// The expected values match Icarus Verilog.
primitive dffp (q, clk, d);
  output q; reg q;
  input clk, d;
  initial q = 1'b0;
  table
  // clk d : q : q+
     r   0 : ? : 0 ;
     r   1 : ? : 1 ;
     f   ? : ? : - ;
     ?   * : ? : - ;
  endtable
endprimitive

module tb;
  reg clk, d, e, v;
  wire [1:0] w;
  dffp u0 (w[0], clk, d);       // The only driver of `w[0]`.
  dffp u1 (w[1], clk, d);       // `w[1]` also has a tristate driver.
  assign w[1] = e ? v : 1'bz;

  initial begin
    clk = 0; d = 1; e = 0; v = 0;
    #1 clk = 1;
    #1 $vogls_assert_eq(w, 2'b11);
    e = 1; v = 0;
    #1 $vogls_assert_eq(w, 2'bx1);
    e = 0; clk = 0; d = 0;
    #1 clk = 1;
    #1 $vogls_assert_eq(w, 2'b00);
  end
endmodule
