// vogls: mode=four-value-logic
//
// The drivers of an output with module path delays are those of an ordinary net: the output can
// have several drivers or be driven partially.
//
// The expected values match Icarus Verilog.
module multiple(input a, input en, output y);
  assign y = a;
  assign y = en ? 1'b0 : 1'bz;
  specify (a => y) = 1; endspecify
endmodule

module partial(input a, output [1:0] y);
  assign y[0] = a;
  specify (a *> y) = 1; endspecify
endmodule

module tb;
  reg a, en;
  wire y1;
  wire [1:0] y2;
  multiple m (a, en, y1);
  partial p (a, y2);

  initial begin
    a = 1; en = 0;
    #5;
    $vogls_assert_eq(y1, 1'b1);
    $vogls_assert_eq(y2, 2'bz1);
    en = 1;
    #5;
    $vogls_assert_eq(y1, 1'bx);
  end
endmodule
