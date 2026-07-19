module test;
   reg clk = 0;
   reg sel = 0;
   reg [64:0] acc;
   reg [31:0] in32;
   wire [64:0] acc_in = sel ? (acc | {33'b0, in32}) : acc;
   always #5 clk = ~clk;
   always @(posedge clk) acc <= acc_in;
   initial begin
      acc = 65'b0;
      in32 = 32'hDEADBEEF;
      #10 sel = 1;
      #50 $finish;
   end
endmodule
