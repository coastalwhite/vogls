module top_module (
    input [99:0] in,
	output [99:0] out
);
	integer i = 0;
	always @(*) for (i = 0; i < 100; i = i + 1) out[99-i] = in[i];
endmodule

module tb();
    reg [99:0] in;
    wire [99:0] out;

    top_module m(in, out);

    initial begin
        #1 in = 0;
        #1 $vogls_assert_eq(out, 0); in = 1;
        #1 $vogls_assert_eq(out, 100'b1 << 99); in = 100'h1_1234_5678_aaaa_bbbb_cccc_dddd;
        #1 $vogls_assert_eq(out, 100'hb_bbb3_333d_ddd5_5551_e6a2_c488); in = 100'ha_bcde_f011_2345_678a_aaab_bbbc;
        #1 $vogls_assert_eq(out, 100'h3_dddd_5555_1e6a_2c48_80f7_b3d5);
    end
endmodule
