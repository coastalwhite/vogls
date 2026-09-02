module top;
  reg u;
  reg [2:0] s;
  reg [7:0] r;
  initial begin
    u = 1'b1;
    s = 3'd2;
    case (u <<< s)
      3'h0: r = 8'd0;
      3'h4: r = 8'd99;
      default: r = 8'd99;
    endcase
    $vogls_assert_eq(r, 99);
  end
endmodule
