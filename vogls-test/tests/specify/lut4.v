// vogls: time=5000
module tb();
    wire out;
    SB_LUT4 #(
      .LUT_INIT(16'h000f)
    ) ctr_SB_LUT4_I3 (
        .I0(1'h0),
        .I1(1'h0),
        .I2(1'h0),
        .I3(1'h0),
        .O(out)
    );
    initial #2000 $vogls_assert_eq(out, 1);
endmodule

// Adapted from the Yosys ICE40 technology map
module SB_LUT4 (
	output O,
	input I0,
	input I1,
	input I2,
	input I3
);
	parameter [15:0] LUT_INIT = 0;
	wire [7:0] s3 = I3 ? LUT_INIT[15:8] : LUT_INIT[7:0];
	wire [3:0] s2 = I2 ? LUT_INIT[ 7:4] : LUT_INIT[3:0];
	wire [1:0] s1 = I1 ?       s2[ 3:2] :       s2[1:0];
	assign O = I0 ? s1[1] : s1[0];
	specify
		(I0 => O) = (1245, 1285);
		(I1 => O) = (1179, 1232);
		(I2 => O) = (1179, 1205);
		(I3 => O) = (861, 874);
	endspecify
endmodule
