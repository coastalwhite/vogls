`default_nettype none
module top_module(
    input wire [15:0] in,
    output wire [7:0] out_hi,
    output wire [7:0] out_lo );

    assign out_hi = in[15:8];
    assign out_lo = in[7:0];
endmodule

module tb();
    reg [15:0] in;
    wire [7:0] out_hi;
    wire [7:0] out_lo;

    top_module m(in, out_hi, out_lo);

    initial begin
        #1 in = 16'hAABB;
        #1 
        $vogls_assert_eq(out_hi, 8'hAA);
        $vogls_assert_eq(out_lo, 8'hBB);
        
        #1 in = 16'h0001;
        #1
        $vogls_assert_eq(out_hi, 8'h00);
        $vogls_assert_eq(out_lo, 8'h01);
    end
endmodule
