module tb();
    reg clk, enable, reset, data_i;
    wire data_o;

    always begin #0 clk = 0; #5000; clk = 1; #5000; end

	SB_DFFESR dff(
		.Q(data_o),
		.C(clk),
		.E(enable),
		.R(reset),
		.D(data_i)
	);

    initial begin
        #0
        enable = 0;
        reset = 0;
        data_i = 0;

        #1000;
        enable = 1;
        reset = 1;

        #10_000;
        $vogls_assert_eq(data_o, 0);
        reset = 0;
        data_i = 1;

        #10_000;
        $vogls_assert_eq(data_o, 1);
        enable = 0; data_i = 0;

        #10_000;
        $vogls_assert_eq(data_o, 1);
        enable = 1;

        #10_000;
        $vogls_assert_eq(data_o, 0);
        $finish();
    end
endmodule

// Adapted from the Yosys ICE40 technology map
module SB_DFFESR (
	output reg Q,
	input C,
	input E,
	input R,
	input D
);
	always @(posedge C)
		if (E) begin
			if (R)
				Q <= 0;
			else
				Q <= D;
		end

	specify
		if (E &&  R) (posedge C => (Q : 1'b0)) = 1391;
		if (E && !R) (posedge C => (Q : D)) = 1391;
	endspecify
endmodule
