// vogls: tlm=tb

`ifdef AES_IMPL_COMP

`include "../../../submodules/aoki-aes/AES_Comp.v"
`define AESMODULE AES_Comp_ENC
`define KIN

`elsif AES_IMPL_PPRM1

`include "../../../submodules/aoki-aes/AES_PPRM1.v"
`define AESMODULE AES_PPRM1_ENC
`define KIN

`elsif AES_IMPL_PPRM3

`include "../../../submodules/aoki-aes/AES_PPRM3.v"
`define AESMODULE AES_PPRM3_ENC
`define KIN

`elsif AES_IMPL_TBL

`include "../../../submodules/aoki-aes/AES_TBL.v"
`define AESMODULE AES_TBL_ENC
`define KIN

`else

`include "../../../submodules/aoki-aes/AES.v"
`define AESMODULE AES_ENC

`endif

module tb();
	reg [127:0] Din, Key;
	reg clk, rstn, Drdy, Krdy, EN, BSY;
	wire [127:0] Dout;
	wire Dvld, bsy;

  	`AESMODULE aes(
      .Din(Din),  
`ifdef KIN
      .Kin(Key),
`else
      .Key(Key),  
`endif
      .Dout(Dout),
      .Drdy(Drdy),
      .Krdy(Krdy),
      .RSTn(rstn),
      .EN(EN),
      .CLK(clk),
      .BSY(BSY),
      .Dvld(Dvld)
  	);

	always begin clk = 0; #5 clk = 1; #5 ; end

	initial begin
		rstn = 0;
		#20
		rstn = 1;
		EN = 1;
		Din = 0; Drdy = 1;
		#10
		Drdy = 0;
		Key = 0; Krdy = 1;
		#10
		Krdy = 0;

		#120

		$vogls_assert_eq(Dout, 128'h66e94bd4ef8a2c3b884cfa59ca342b2e);
        $finish();
	end
endmodule
