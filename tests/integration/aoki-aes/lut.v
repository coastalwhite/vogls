// vogls: tlm=tb

`timescale 1fs / 1fs
`ifdef AES_IMPL_COMP

`include "../../../submodules/aoki-aes/AES_PPRM1.v"
`define AESMODULE AES_PPRM1_ENC
`define KIN 1

`else

`include "../../../submodules/aoki-aes/AES.v"
`define AESMODULE AES_ENC

`endif

module tb();
	reg [127:0] Din, Key;
	reg clk, rstn, Drdy, Krdy, EN;
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
      .BSY(bsy),
      .Dvld(Dvld)
  	);

	always begin clk = 0; #5 clk = 1; #5 ; end

	initial begin
		rstn = 0;
		#20
		rstn = 1;
		EN = 1;
		Krdy = 1;
		#20
		Krdy = 0;
		Drdy = 1;
		#20
		Drdy = 0;

		#200
        $finish();
	end
endmodule
