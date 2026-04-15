// vogls: time=none

module tb();
	localparam PERIOD = 100_000;

	reg CLK;
	reg RST_N;
	reg TX;
	wire RX;

	task test_case(input [1:0] a_i, b_i, z_o);
		reg [1:0] z_o_tmp;
	begin
		#(PERIOD / 2) CLK = 1'b1;
		#(PERIOD / 2) CLK = 1'b0;
		TX = a_i[0];
		RST_N = 1'b1;

		#(PERIOD / 2) CLK = 1'b1;
		#(PERIOD / 2) CLK = 1'b0;
		TX = a_i[1];

		#(PERIOD / 2) CLK = 1'b1;
		#(PERIOD / 2) CLK = 1'b0;
		TX = b_i[0];
		
		#(PERIOD / 2) CLK = 1'b1;
		#(PERIOD / 2) CLK = 1'b0;
		TX = b_i[1];

		#(PERIOD / 2) CLK = 1'b1;
		#(PERIOD / 2) CLK = 1'b0;
		#(PERIOD / 2) CLK = 1'b1;
		#(PERIOD / 2) CLK = 1'b0;
		#(PERIOD / 2) CLK = 1'b1;
		#(PERIOD / 2) CLK = 1'b0;
		#(PERIOD / 2) CLK = 1'b1;
		#(PERIOD / 2) CLK = 1'b0;
		z_o_tmp[0] = RX;
		#(PERIOD / 2) CLK = 1'b1;
		#(PERIOD / 2) CLK = 1'b0;
		z_o_tmp[1] = RX;

        $vogls_assert_eq(z_o_tmp, z_o);

		RST_N = 1'b0;
		#(PERIOD / 2) CLK = 1'b1;
		#(PERIOD / 2) CLK = 1'b0;
	end
	endtask

    initial begin
		RST_N = 1'b0;
		CLK = 1'b0;
		TX = 1'b0;
		RX = 1'b0;

		test_case(2'b00, 2'b00, 2'b00);
		test_case(2'b01, 2'b00, 2'b00);
		test_case(2'b10, 2'b00, 2'b00);
		test_case(2'b11, 2'b00, 2'b00);
		test_case(2'b00, 2'b01, 2'b00);
		test_case(2'b01, 2'b01, 2'b10);
		test_case(2'b10, 2'b01, 2'b11);
		test_case(2'b11, 2'b01, 2'b01);
		test_case(2'b00, 2'b10, 2'b00);
		test_case(2'b01, 2'b10, 2'b11);
		test_case(2'b10, 2'b10, 2'b01);
		test_case(2'b11, 2'b10, 2'b10);
		test_case(2'b00, 2'b11, 2'b00);
		test_case(2'b01, 2'b11, 2'b01);
		test_case(2'b10, 2'b11, 2'b10);
		test_case(2'b11, 2'b11, 2'b11);

		$finish();
    end

	top t(CLK, RST_N, TX, RX);
endmodule

`ifndef RTL
module top(CLK, BTN_N, RX, TX);
  input CLK;
  wire CLK;
  input BTN_N;
  wire BTN_N;
  input RX;
  wire RX;
  output TX;
  wire TX;
  wire BTN_N_SB_LUT4_I3_O;
  wire [3:0] RX_SB_LUT4_I0_1_I3;
  wire [2:0] RX_SB_LUT4_I0_1_O;
  wire [3:0] RX_SB_LUT4_I0_I3;
  wire [2:0] RX_SB_LUT4_I0_O;
  wire TX_SB_DFFE_Q_D;
  wire [3:0] TX_SB_DFFE_Q_D_SB_LUT4_O_I1;
  wire TX_SB_DFFE_Q_E;
  wire [1:0] a_i;
  wire a_i_SB_DFFSR_Q_1_D;
  wire a_i_SB_DFFSR_Q_D;
  wire [1:0] b_i;
  wire b_i_SB_DFFSR_Q_1_D;
  wire b_i_SB_DFFSR_Q_D;
  wire [3:0] b_i_SB_LUT4_I2_O;
  wire [3:0] b_i_SB_LUT4_I3_O;
  wire [2:0] ctr;
  wire ctr_SB_DFFSR_Q_1_D;
  wire ctr_SB_DFFSR_Q_D;
  wire enable;
  wire enable_SB_DFFSR_Q_D;
  wire enable_SB_DFFSR_Q_D_SB_LUT4_O_I3;
  SB_LUT4 #(
    .LUT_INIT(16'hc000)
  ) BTN_N_SB_LUT4_I1 (
    .I0(1'h0),
    .I1(BTN_N),
    .I2(ctr[1]),
    .I3(enable),
    .O(TX_SB_DFFE_Q_E)
  );
  SB_LUT4 #(
    .LUT_INIT(16'h00ff)
  ) BTN_N_SB_LUT4_I3 (
    .I0(1'h0),
    .I1(1'h0),
    .I2(1'h0),
    .I3(BTN_N),
    .O(BTN_N_SB_LUT4_I3_O)
  );
  SB_LUT4 #(
    .LUT_INIT(16'hf704)
  ) RX_SB_LUT4_I0 (
    .I0(RX),
    .I1(ctr[1]),
    .I2(enable),
    .I3(RX_SB_LUT4_I0_I3[3]),
    .O(RX_SB_LUT4_I0_O[2])
  );
  SB_LUT4 #(
    .LUT_INIT(16'hfe02)
  ) RX_SB_LUT4_I0_1 (
    .I0(RX),
    .I1(ctr[1]),
    .I2(enable),
    .I3(RX_SB_LUT4_I0_1_I3[3]),
    .O(RX_SB_LUT4_I0_1_O[2])
  );
  SB_LUT4 #(
    .LUT_INIT(16'hcfc0)
  ) RX_SB_LUT4_I0_1_I3_SB_LUT4_O (
    .I0(1'h0),
    .I1(a_i[1]),
    .I2(ctr[0]),
    .I3(a_i[0]),
    .O(RX_SB_LUT4_I0_1_I3[3])
  );
  SB_LUT4 #(
    .LUT_INIT(16'h303f)
  ) RX_SB_LUT4_I0_I3_SB_LUT4_O (
    .I0(1'h0),
    .I1(b_i[1]),
    .I2(ctr[0]),
    .I3(b_i[0]),
    .O(RX_SB_LUT4_I0_I3[3])
  );
  SB_DFFE TX_SB_DFFE_Q (
    .C(CLK),
    .D(TX_SB_DFFE_Q_D),
    .E(TX_SB_DFFE_Q_E),
    .Q(TX)
  );
  SB_LUT4 #(
    .LUT_INIT(16'haa20)
  ) TX_SB_DFFE_Q_D_SB_LUT4_O (
    .I0(enable),
    .I1(TX_SB_DFFE_Q_D_SB_LUT4_O_I1[1]),
    .I2(TX_SB_DFFE_Q_D_SB_LUT4_O_I1[2]),
    .I3(TX_SB_DFFE_Q_D_SB_LUT4_O_I1[3]),
    .O(TX_SB_DFFE_Q_D)
  );
  SB_DFFSR a_i_SB_DFFSR_Q (
    .C(CLK),
    .D(a_i_SB_DFFSR_Q_D),
    .Q(a_i[1]),
    .R(BTN_N_SB_LUT4_I3_O)
  );
  SB_DFFSR a_i_SB_DFFSR_Q_1 (
    .C(CLK),
    .D(a_i_SB_DFFSR_Q_1_D),
    .Q(a_i[0]),
    .R(BTN_N_SB_LUT4_I3_O)
  );
  SB_LUT4 #(
    .LUT_INIT(16'hf3c0)
  ) a_i_SB_DFFSR_Q_1_D_SB_LUT4_O (
    .I0(1'h0),
    .I1(ctr[0]),
    .I2(a_i[0]),
    .I3(RX_SB_LUT4_I0_1_O[2]),
    .O(a_i_SB_DFFSR_Q_1_D)
  );
  SB_LUT4 #(
    .LUT_INIT(16'hfc0c)
  ) a_i_SB_DFFSR_Q_D_SB_LUT4_O (
    .I0(1'h0),
    .I1(a_i[1]),
    .I2(ctr[0]),
    .I3(RX_SB_LUT4_I0_1_O[2]),
    .O(a_i_SB_DFFSR_Q_D)
  );
  SB_DFFSR b_i_SB_DFFSR_Q (
    .C(CLK),
    .D(b_i_SB_DFFSR_Q_D),
    .Q(b_i[1]),
    .R(BTN_N_SB_LUT4_I3_O)
  );
  SB_DFFSR b_i_SB_DFFSR_Q_1 (
    .C(CLK),
    .D(b_i_SB_DFFSR_Q_1_D),
    .Q(b_i[0]),
    .R(BTN_N_SB_LUT4_I3_O)
  );
  SB_LUT4 #(
    .LUT_INIT(16'hc0f3)
  ) b_i_SB_DFFSR_Q_1_D_SB_LUT4_O (
    .I0(1'h0),
    .I1(ctr[0]),
    .I2(b_i[0]),
    .I3(RX_SB_LUT4_I0_O[2]),
    .O(b_i_SB_DFFSR_Q_1_D)
  );
  SB_LUT4 #(
    .LUT_INIT(16'h0cfc)
  ) b_i_SB_DFFSR_Q_D_SB_LUT4_O (
    .I0(1'h0),
    .I1(b_i[1]),
    .I2(ctr[0]),
    .I3(RX_SB_LUT4_I0_O[2]),
    .O(b_i_SB_DFFSR_Q_D)
  );
  SB_LUT4 #(
    .LUT_INIT(16'he300)
  ) b_i_SB_LUT4_I0 (
    .I0(b_i[1]),
    .I1(a_i[1]),
    .I2(ctr[0]),
    .I3(a_i[0]),
    .O(TX_SB_DFFE_Q_D_SB_LUT4_O_I1[1])
  );
  SB_LUT4 #(
    .LUT_INIT(16'h0028)
  ) b_i_SB_LUT4_I0_1 (
    .I0(b_i[1]),
    .I1(a_i[0]),
    .I2(b_i_SB_LUT4_I3_O[2]),
    .I3(b_i_SB_LUT4_I3_O[3]),
    .O(TX_SB_DFFE_Q_D_SB_LUT4_O_I1[3])
  );
  SB_LUT4 #(
    .LUT_INIT(16'hc0c8)
  ) b_i_SB_LUT4_I1 (
    .I0(a_i[1]),
    .I1(b_i[0]),
    .I2(a_i[0]),
    .I3(b_i_SB_LUT4_I2_O[3]),
    .O(TX_SB_DFFE_Q_D_SB_LUT4_O_I1[2])
  );
  SB_LUT4 #(
    .LUT_INIT(16'h00f0)
  ) b_i_SB_LUT4_I2 (
    .I0(1'h0),
    .I1(1'h0),
    .I2(b_i[1]),
    .I3(ctr[0]),
    .O(b_i_SB_LUT4_I2_O[3])
  );
  SB_LUT4 #(
    .LUT_INIT(16'h3c00)
  ) b_i_SB_LUT4_I3 (
    .I0(1'h0),
    .I1(a_i[1]),
    .I2(ctr[0]),
    .I3(b_i[0]),
    .O(b_i_SB_LUT4_I3_O[3])
  );
  SB_LUT4 #(
    .LUT_INIT(16'h00f0)
  ) b_i_SB_LUT4_I3_O_SB_LUT4_O (
    .I0(1'h0),
    .I1(1'h0),
    .I2(a_i[1]),
    .I3(ctr[0]),
    .O(b_i_SB_LUT4_I3_O[2])
  );
  SB_DFFSR ctr_SB_DFFSR_Q (
    .C(CLK),
    .D(ctr_SB_DFFSR_Q_D),
    .Q(ctr[1]),
    .R(BTN_N_SB_LUT4_I3_O)
  );
  SB_DFFSR ctr_SB_DFFSR_Q_1 (
    .C(CLK),
    .D(ctr_SB_DFFSR_Q_1_D),
    .Q(ctr[0]),
    .R(BTN_N_SB_LUT4_I3_O)
  );
  SB_LUT4 #(
    .LUT_INIT(16'h00ff)
  ) ctr_SB_DFFSR_Q_1_D_SB_LUT4_O (
    .I0(1'h0),
    .I1(1'h0),
    .I2(1'h0),
    .I3(ctr[0]),
    .O(ctr_SB_DFFSR_Q_1_D)
  );
  SB_LUT4 #(
    .LUT_INIT(16'h6996)
  ) ctr_SB_DFFSR_Q_D_SB_LUT4_O (
    .I0(1'h0),
    .I1(1'h0),
    .I2(ctr[1]),
    .I3(ctr[0]),
    .O(ctr_SB_DFFSR_Q_D)
  );
  SB_DFFSR enable_SB_DFFSR_Q (
    .C(CLK),
    .D(enable_SB_DFFSR_Q_D),
    .Q(enable),
    .R(BTN_N_SB_LUT4_I3_O)
  );
  SB_LUT4 #(
    .LUT_INIT(16'h6996)
  ) enable_SB_DFFSR_Q_D_SB_LUT4_O (
    .I0(1'h0),
    .I1(1'h0),
    .I2(enable),
    .I3(enable_SB_DFFSR_Q_D_SB_LUT4_O_I3),
    .O(enable_SB_DFFSR_Q_D)
  );
  SB_CARRY enable_SB_DFFSR_Q_D_SB_LUT4_O_I3_SB_CARRY_CO (
    .CI(ctr[0]),
    .CO(enable_SB_DFFSR_Q_D_SB_LUT4_O_I3),
    .I0(1'h0),
    .I1(ctr[1])
  );
  assign b_i_SB_LUT4_I3_O[1:0] = { a_i[0], b_i[1] };
  assign RX_SB_LUT4_I0_I3[2:0] = { enable, ctr[1], RX };
  assign TX_SB_DFFE_Q_D_SB_LUT4_O_I1[0] = enable;
  assign RX_SB_LUT4_I0_1_I3[2:0] = { enable, ctr[1], RX };
  assign b_i_SB_LUT4_I2_O[2:0] = { a_i[0], b_i[0], a_i[1] };
  assign RX_SB_LUT4_I0_O[1:0] = { ctr[0], b_i[1] };
  assign RX_SB_LUT4_I0_1_O[1:0] = { ctr[0], a_i[1] };
  assign ctr[2] = enable;
endmodule

module SB_LUT4 (
	output O,
	input I0,
	input I1,
	input I2,
	input I3
);
	parameter [15:0] LUT_INIT = 0;
	wire [7:0] s3 = I3 ? LUT_INIT[15:8] : LUT_INIT[7:0];
	wire [3:0] s2 = I2 ?       s3[ 7:4] :       s3[3:0];
	wire [1:0] s1 = I1 ?       s2[ 3:2] :       s2[1:0];
	assign O = I0 ? s1[1] : s1[0];
`ifndef NO_SPECIFY
	specify
		(I0 => O) = (1245, 1285);
		(I1 => O) = (1179, 1232);
		(I2 => O) = (1179, 1205);
		(I3 => O) = (861, 874);
	endspecify
`endif
endmodule

module SB_CARRY (output CO, input I0, I1, CI);
	assign CO = (I0 && I1) || ((I0 || I1) && CI);
`ifndef NO_SPECIFY
	specify
		(CI => CO) = (278, 278);
		(I0 => CO) = (675, 662);
		(I1 => CO) = (609, 358);
	endspecify
`endif
endmodule
module SB_CARRY (output CO, input I0, I1, CI);
	assign CO = (I0 && I1) || ((I0 || I1) && CI);
`ifndef NO_SPECIFY
	specify
		(CI => CO) = (278, 278);
		(I0 => CO) = (675, 662);
		(I1 => CO) = (609, 358);
	endspecify
`endif
endmodule

module SB_DFFSR (
	output reg Q,
	input C, R, D
);
	always @(posedge C)
		if (R)
			Q <= 0;
		else
			Q <= D;
`ifndef NO_SPECIFY
	specify
		// https://github.com/YosysHQ/icestorm/blob/95949315364f8d9b0c693386aefadf44b28e2cf6/icefuzz/timings_lp1k.txt#L86
		//   minus https://github.com/YosysHQ/icestorm/blob/95949315364f8d9b0c693386aefadf44b28e2cf6/icefuzz/timings_lp1k.txt#L80
		$setup(D, posedge C, /*1232 - 1285*/ 0); // Negative times not currently supported
		// https://github.com/YosysHQ/icestorm/blob/95949315364f8d9b0c693386aefadf44b28e2cf6/icefuzz/timings_up5k.txt#L90
		$setup(R, posedge C, 530);
		// https://github.com/YosysHQ/icestorm/blob/95949315364f8d9b0c693386aefadf44b28e2cf6/icefuzz/timings_up5k.txt#L102
		if ( R) (posedge C => (Q : 1'b0)) = 1391;
		if (!R) (posedge C => (Q : D)) = 1391;
	endspecify
`endif
endmodule

module SB_DFFE (
	output reg Q,
	input C,
	input E,
	input D
);
	always @(posedge C)
		if (E)
			Q <= D;
`ifndef NO_SPECIFY
	specify
		$setup(D, posedge C &&& E, /*1232 - 1285*/ 0); // Negative times not currently supported
		$setup(E, posedge C, 0);
		if (E) (posedge C => (Q : D)) = 1391;
	endspecify
`endif
endmodule
`else
module top(
	input wire  CLK,
    input wire  BTN_N,
    input wire  RX,
    output reg TX
);
	reg [1:0] a_i;
	reg [1:0] b_i;
	reg [2:0] ctr;

	wire [1:0] z_o;
	wire enable = ctr >= 4'h4;

	always @ (posedge CLK) begin
		if (~BTN_N) begin
			a_i <= 2'b0;
			b_i <= 2'b0;
			ctr <= 3'b0;
		end else begin
			a_i[ctr[0]] <= (ctr>=4'h0 && ctr<4'h2) ? RX : a_i[ctr[0]];
			b_i[ctr[0]] <= (ctr>=4'h2 && ctr<4'h4) ? RX : b_i[ctr[0]];
			TX          <= (ctr>=4'h6) ? z_o[ctr[0]] : TX;
			ctr         <=  ctr + 1;
		end
	end

	aes_mul_gf2p2 m(a_i & {2{enable}}, b_i & {2{enable}}, z_o);
endmodule

module aes_mul_gf2p2(a_i, b_i, z_o);
    input [1:0]  a_i;
    input [1:0]  b_i;
    output [1:0] z_o;

    wire a, b, c;

    assign a = a_i[1] & b_i[1];
    assign b = ^a_i & ^b_i;
    assign c = a_i[0] & b_i[0];

    assign z_o = { a ^ b, c ^ b };
endmodule
`endif
