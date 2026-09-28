`ifdef AES_IMPL_COMP

`include "../submodules/aoki-aes/AES_Comp.v"
`define AESMODULE AES_Comp_ENC
`define KIN

`elsif AES_IMPL_PPRM1

`include "../submodules/aoki-aes/AES_PPRM1.v"
`define AESMODULE AES_PPRM1_ENC
`define KIN

`elsif AES_IMPL_PPRM3

`include "../submodules/aoki-aes/AES_PPRM3.v"
`define AESMODULE AES_PPRM3_ENC
`define KIN

`elsif AES_IMPL_TBL

`include "../submodules/aoki-aes/AES_TBL.v"
`define AESMODULE AES_TBL_ENC
`define KIN

`else

`include "../submodules/aoki-aes/AES.v"
`define AESMODULE AES_ENC

`endif

`default_nettype none

module aes_wrap(
    input clk,
    input rstn,
    input receive_enable,

    input  [7:0] rx_byte,

    input  is_transmitting,
    output wire transmit_enable,
    output wire [7:0] tx_byte
);
  localparam S_RCV_KEY  = 3'b000;
  localparam S_SET_KEY  = 3'b001;
  localparam S_RCV_DATA = 3'b010;
  localparam S_SET_DATA = 3'b011;
  localparam S_BSY      = 3'b100;
  localparam S_SND_DATA = 3'b101;
  localparam S_SND_WAIT = 3'b110;

  reg [127:0] Din, Key;
  wire [127:0] Dout;

  reg [2:0] state;
  reg [3:0] ctr;

  assign transmit_enable = state == S_SND_DATA;
  assign tx_byte = Dout[ 8*ctr +: 8];

  wire Dvld;

  `AESMODULE aes(
      .Din(Din),  
`ifdef KIN
      .Kin(Key),
`else
      .Key(Key),  
`endif
      .Dout(Dout),
      .Drdy(state == S_SET_DATA),
      .Krdy(state == S_SET_KEY),
      .RSTn(rstn),
      .EN(state == S_SET_KEY || state == S_SET_DATA || state == S_BSY),
      .CLK(clk),
      .BSY(),
      .Dvld(Dvld)
  );

  always @ (posedge clk) begin
      if (~rstn) begin
          state <= 0;
          ctr   <= 0;
          Din   <= 0;
          Key   <= 0;
      end else begin
          case (state)
              S_RCV_KEY: if (receive_enable) begin
                  ctr <= ctr + 1;
                  Key[ ctr*8 +: 8] <= rx_byte;
                  if (ctr == 15) state <= S_SET_KEY;
              end
              S_SET_KEY: state <= S_RCV_DATA;
              S_RCV_DATA: if (receive_enable) begin
                  ctr <= ctr + 1;
                  Din[ ctr*8 +: 8] <= rx_byte;
                  if (ctr == 15) state <= S_SET_DATA;
              end
              S_SET_DATA: state <= S_BSY;
              S_BSY: if (Dvld) state <= S_SND_DATA;
              S_SND_DATA: state <= S_SND_WAIT;
              S_SND_WAIT: if (~is_transmitting) begin
                  ctr <= ctr + 1;
                  if (ctr == 15) state <= S_RCV_DATA;
                  else           state <= S_SND_DATA;
              end
          endcase
      end
  end
endmodule

`default_nettype wire
