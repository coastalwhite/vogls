module tb();
    localparam [31:0] a = 32'h3;
	reg [62:0] b;

    initial begin
        b = a << 31;
        $vogls_assert_eq(b, 63'h1_8000_0000);
    end
endmodule
