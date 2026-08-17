// vogls: panic
`timescale 1ns/1ps
// The `celldefine / `endcelldefine directives (emitted by most standard-cell
// Verilog libraries, e.g. NangateOpenCellLibrary.v) hit a todo!() in the
// tokenizer and panic instead of being ignored like other unsupported
// directives.
`celldefine
module cell;
   reg [7:0] r;
   initial r = 8'd0;
endmodule
`endcelldefine
