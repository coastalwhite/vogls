// vogls: mode=four-value-logic
// vogls: verify-stdout
//
// A four-valued literal with fewer digits than its width is extended with `x` or `z` when its
// leftmost digit is `x` or `z`, and with zeros otherwise. The leftmost digit was repeated instead.
module top;
  reg [39:0] a, b;
  reg [63:0] c;
  reg [99:0] d, e;
  reg [11:0] f;
  reg [71:0] g;
  initial begin
    a = 40'h1x; b = 40'hx1; c = 64'hz1; d = 100'b1z; e = 100'hz_0000_0000_0000_0000_0000_0001;
    f = 12'h1z; g = 72'o1x;
    $display("%b", a);
    $display("%b", b);
    $display("%b", c);
    $display("%b", d);
    $display("%b", e);
    $display("%b %b", f, g);
  end
endmodule
