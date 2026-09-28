// vogls: tlm=tb

`define AESMODULE AES_ENC
`include "../../../submodules/aoki-aes/AES.v"

module tb();
	reg [31:0] x;
    wire [31:0] y;

    SubBytes sb(x, y);

	initial begin
        x = 32'hFFFF_FFFF;
        #0
        x = 32'h0000_0000; #0 $vogls_assert_eq(y, 32'h6363_6363);
        x = 32'h4213_3705; #0 $vogls_assert_eq(y, 32'h2C7D_9A6B);
	end
endmodule
