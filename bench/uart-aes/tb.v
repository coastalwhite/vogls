`ifdef GLS
`define NO_ICE40_DEFAULT_ASSIGNMENTS
`include "../../submodules/fpga-tools/ice40/cells_sim.v"
`include "../../submodules/fpga-tools/components/uart.v"
`include "./build/gtl.v"
`else
`include "./uart-hello.v"
`endif

`default_nettype none
`timescale 1ns / 1ps

module tb();
    localparam CLK_PERIOD = 1000;

	reg clk, nrst;
	wire rx, tx;

	top t(
		.CLK(clk),
		.BTN_N(nrst),
		.RX(rx),
		.TX(tx),
		.LED1(),
		.LED2(),
		.LED3(),
		.LED4()
	);

	reg transmit;
	wire received, is_receiving, is_transmitting, recv_error;
	reg [7:0] tb_dut_byte;
    wire [7:0] dut_tb_byte;

  	uart
  	#(.baud_rate(9600), .sys_clk_freq(12_000_000))
  	tb_uart (
		.clk(clk),                        // The master clock for this module
        .rst(~nrst),                      // Synchronous reset
        .rx(tx),                		  // Incoming serial line
        .tx(rx),                		  // Outgoing serial line
        .transmit(transmit),              // Signal to transmit
        .tx_byte(tb_dut_byte),            // Byte to transmit
        .received(received),              // Indicated that a byte has been received
        .rx_byte(dut_tb_byte),            // Byte received
        .is_receiving(is_receiving),      // Low when receive line is idle
        .is_transmitting(is_transmitting),// Low when transmit line is idle
        .recv_error(recv_error)           // Indicates error in receiving packet.
    );

	always begin clk = 1'b0; #(CLK_PERIOD / 2) clk = 1'b1; #(CLK_PERIOD / 2) ; end

    task send_u128(input [127:0] value);
        integer i;

        for (i = 0; i < 16; i = i + 1) begin
            #(CLK_PERIOD)
            tb_dut_byte = value[8*i +: 8];
            transmit = 1;
            #(CLK_PERIOD)
            transmit = 0;

            wait (is_transmitting == 0);
        end
    endtask

	initial begin
        nrst = 0;

        #(2*CLK_PERIOD)

        nrst = 1;
        transmit = 0;
		tb_dut_byte = 8'h01;

        #(2*CLK_PERIOD)
        send_u128(128'h000000000000000000000000000000);
        #(2*CLK_PERIOD)
        send_u128(128'h0102030405060708090A0B0C0D0E0F);

        begin: xyz
            reg [127:0] out;
            integer i;

            for (i = 0; i < 16; i = i + 1) begin
                wait (received);
                out[8*i +: 8] = dut_tb_byte;
                wait (~received);
            end

            $display("Output = %h", out);
        end
        $finish();
	end
endmodule

`default_nettype wire