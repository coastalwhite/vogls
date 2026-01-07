module top_module (
    input [7:0] in,
    output parity
);
    assign parity = ^in;
endmodule

module tb();
    reg [7:0] in;
    wire parity;

    top_module m(in, parity);

    initial begin
        #1 in = 0;
        #1 $vogls_assert_eq(parity, 0); in = 8'b0000_0001; 
        #1 $vogls_assert_eq(parity, 1); in = 8'b1000_0001; 
        #1 $vogls_assert_eq(parity, 0); in = 8'b1000_1001; 
        #1 $vogls_assert_eq(parity, 1); in = 8'b1000_1001; 
        #1 $vogls_assert_eq(parity, 1); in = 8'b1011_1001; 
        #1 $vogls_assert_eq(parity, 1); in = 8'b1011_1101; 
        #1 $vogls_assert_eq(parity, 0); in = 8'b1111_1111; 
        #1 $vogls_assert_eq(parity, 0); in = 8'b0001_1000; 
        #1 $vogls_assert_eq(parity, 0);
    end
endmodule
