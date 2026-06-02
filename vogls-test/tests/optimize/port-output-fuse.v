module driver (output wire [31:0] out_o);
  assign out_o = 32'h00000020;
endmodule

module top;
  wire [63:0] rf;
  localparam WIDTH = 32;

  driver u(.out_o(rf[31-:WIDTH]));

  initial begin
    #1;
	$vogls_assert_eq(rf[31:0], 32'h00000020);
  end
endmodule
