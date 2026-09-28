// vogls: verify-stdout
module tb();
    integer a;

    initial begin
		a = 32'b01100011011000110110001101100011;
		$display("%b", a);
		a = 32'b0;
		$display("%0b", a);
        a = 32'b11000110110001101100011011000110;
		$display("%b", a);

		$display("%b", 6'b101010);
		$display("%b", 6'b001011);
		$display("%0b", 6'b001011);
		$display("%06b", 6'b001011);
		$display("%07b", 6'b001011);
		$display("%05b", 6'b001011);
		$display("%04b", 6'b001011);
		$display("%03b", 6'b001011);
    end
endmodule
