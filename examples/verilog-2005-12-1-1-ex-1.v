// Lower level module:
// module description of a nand flip-flop circuit
module ffnand (q, qbar, preset, clear);
	output q, qbar;
	//declares 2 circuit output nets
	input preset, clear;
	//declares 2 circuit input nets
	// declaration of two nand gates and their interconnections
	nand g1 (q, qbar, preset),
	g2 (qbar, q, clear);
endmodule
// Higher level module:
// a waveform description for the nand flip-flop
module ffnand_wave;
	wire out1, out2;
	//outputs from the circuit
	reg in1, in2;
	//variables to drive the circuit
	parameter d = 10;
	// instantiate the circuit ffnand, name it "ff",
	// and specify the IO port interconnections
	ffnand ff(out1, out2, in1, in2);
	// define the waveform to stimulate the circuit
	initial begin
		#d in1 = 0; in2 = 1;
		#d in1 = 1;
		#d in2 = 0;
		#d in2 = 1;
	end
endmodule