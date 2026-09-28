`include "../submodules/fpga-tools/components/uart.v"
`include "./aes_wrap.v"

`default_nettype none

module top(
	input wire  CLK,
    input wire  BTN_N,
    input wire  RX,
    output wire TX,
    output wire LED1,
    output wire LED2,
	output wire LED3,
    output wire LED4
);

  parameter baud_rate = 9600;

  wire       nrst = BTN_N;
  wire       transmit;
  wire [7:0] tx_byte;
  wire       received;
  wire [7:0] rx_byte;
  wire       is_receiving;
  wire       is_transmitting;
  wire       recv_error;

  uart
  	#(.baud_rate(9600), .sys_clk_freq(12_000_000))
  	uart0 (
		.clk(CLK),                        // The master clock for this module
        .rst(!nrst),                      // Synchronous reset
        .rx(RX),                		  // Incoming serial line
        .tx(TX),                		  // Outgoing serial line
        .transmit(transmit),              // Signal to transmit
        .tx_byte(tx_byte),                // Byte to transmit
        .received(received),              // Indicated that a byte has been received
        .rx_byte(rx_byte),                // Byte received
        .is_receiving(is_receiving),      // Low when receive line is idle
        .is_transmitting(is_transmitting),// Low when transmit line is idle
        .recv_error(recv_error)           // Indicates error in receiving packet.
    );


  aes_wrap aes_wrap(
    .clk(CLK),
    .rstn(nrst),
    .receive_enable(received),
    .rx_byte(rx_byte),
    .is_transmitting(is_transmitting),
    .transmit_enable(transmit),
    .tx_byte(tx_byte)
  );

  assign LED1 = 1'bx;
  assign LED2 = 1'bx;
  assign LED3 = is_receiving;
  assign LED4 = is_transmitting;
endmodule

`default_nettype wire