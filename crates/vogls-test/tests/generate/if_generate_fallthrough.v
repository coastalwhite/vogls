// vogls: verify-stdout
`timescale 1ns/1ps
module mux #(
  parameter AsyncOn  = 1,
  parameter EnSecBuf = 0
) (
  input  [3:0] a,
  output [3:0] q
);
  generate
    if (EnSecBuf) begin : branch_sec
      assign q = 4'hA;
    end
    else if (!AsyncOn) begin : branch_no_async
      assign q = 4'hB;
    end
    else begin : branch_feedthru
      assign q = a;
    end
  endgenerate
endmodule

module top;
  wire [3:0] q;
  mux #(.AsyncOn(1), .EnSecBuf(0)) u (.a(4'h6), .q(q));
  initial begin
    #1;
    $display("q = %h (expect 6)", q);
  end
endmodule
