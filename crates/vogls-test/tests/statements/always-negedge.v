// vogls: mode=four-value-logic
module top;
  reg rst_n;
  reg [7:0] q = 8'hFF;

  always @(negedge rst_n)
    q <= 8'h00;

  initial begin
    #5;
    rst_n = 0;
    #5;
    $vogls_assert_eq(q, 8'h00);
  end
endmodule
