module top_module ( 
    input [2:0] sel, 
    input [3:0] data0,
    input [3:0] data1,
    input [3:0] data2,
    input [3:0] data3,
    input [3:0] data4,
    input [3:0] data5,
    output reg [3:0] out   );

    always @(*) begin
        case (sel)
            3'd0: out = data0;
            3'd1: out = data1;
            3'd2: out = data2;
            3'd3: out = data3;
            3'd4: out = data4;
            3'd5: out = data5;
            default: out = 4'h0;
        endcase
    end

endmodule

module tb();
    reg [2:0] sel;
    reg [3:0] data0, data1, data2, data3, data4, data5;
    wire [3:0] out;

    top_module m(sel, data0, data1, data2, data3, data4, data5, out);

    initial begin
        #0
        sel = 3'd0;
        data0 = 4'h9; data1 = 4'hA; data2 = 4'hB;
        data3 = 4'hC; data4 = 4'hD; data5 = 4'hF;
        #1 $vogls_assert_eq(out, 4'h9); sel = 3'd1;
        #1 $vogls_assert_eq(out, 4'hA); sel = 3'd2;
        #1 $vogls_assert_eq(out, 4'hB); sel = 3'd3;
        #1 $vogls_assert_eq(out, 4'hC); sel = 3'd4;
        #1 $vogls_assert_eq(out, 4'hD); sel = 3'd5;
        #1 $vogls_assert_eq(out, 4'hF); sel = 3'd6;
        #1 $vogls_assert_eq(out, 4'h0); sel = 3'd7;
        #1 $vogls_assert_eq(out, 4'h0); sel = 3'd0;
        #1 $vogls_assert_eq(out, 4'h9);
    end
endmodule
