// vogls: panic
module top;
  localparam W = 4;
  reg [7:0] r;

  initial begin
    r = 8'ha5;
    if (((2 * W) % W) > 0)
      r[0+:(2 * W) % W] = 1'b0;
    $display("r = %b", r);
    $finish;
  end
endmodule
