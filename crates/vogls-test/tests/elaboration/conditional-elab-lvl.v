module top;
  parameter Enable = 1;
  wire [3:0] a;

  generate
    if (Enable) begin : g
      localparam [31:0] W = 4;
      wire [W-1:0] t;
      assign t = 4'ha;
      assign a = t;
    end
  endgenerate

  initial begin
    #0 $vogls_assert_eq(a, 4'ha);
  end
endmodule
